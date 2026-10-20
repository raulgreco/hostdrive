use crate::projects::ProjectRepository;
use rusqlite::OptionalExtension;
use serde::Serialize;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalEvent {
    id: i64,
    direction: String,
    text: String,
    created_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    path: String,
    format: String,
    redaction_applied: bool,
}

#[tauri::command]
pub fn get_terminal_session_events(
    repository: State<'_, ProjectRepository>,
    session_id: String,
    limit: Option<u16>,
    offset: Option<u32>,
) -> Result<Vec<TerminalEvent>, String> {
    let connection = repository.open_for_workspace().map_err(db_error)?;
    let exists = connection
        .query_row(
            "SELECT 1 FROM terminal_sessions WHERE id=?1",
            [&session_id],
            |_| Ok(true),
        )
        .optional()
        .map_err(db_error)?
        .unwrap_or(false);
    if !exists {
        return Err("sessão não encontrada".into());
    }
    let mut statement = connection.prepare("SELECT id,direction,data,created_at FROM terminal_events WHERE session_id=?1 ORDER BY id LIMIT ?2 OFFSET ?3").map_err(db_error)?;
    let rows = statement
        .query_map(
            rusqlite::params![
                session_id,
                limit.unwrap_or(200).clamp(1, 500),
                offset.unwrap_or(0)
            ],
            |row| {
                let data: Vec<u8> = row.get(2)?;
                Ok(TerminalEvent {
                    id: row.get(0)?,
                    direction: row.get(1)?,
                    text: strip_ansi(&String::from_utf8_lossy(&data)),
                    created_at: row.get(3)?,
                })
            },
        )
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(rows)
}

#[tauri::command]
pub fn export_terminal_session(
    repository: State<'_, ProjectRepository>,
    session_id: String,
    format: String,
) -> Result<ExportResult, String> {
    if !["md", "json", "txt"].contains(&format.as_str()) {
        return Err("formato de exportação inválido".into());
    }
    let connection = repository.open_for_workspace().map_err(db_error)?;
    let header: (String, String, i64) = connection
        .query_row(
            "SELECT agent_id,working_directory,started_at FROM terminal_sessions WHERE id=?1",
            [&session_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(db_error)?;
    let mut statement = connection
        .prepare(
            "SELECT direction,data,created_at FROM terminal_events WHERE session_id=?1 ORDER BY id",
        )
        .map_err(db_error)?;
    let events = statement
        .query_map([&session_id], |row| {
            let data: Vec<u8> = row.get(1)?;
            Ok((
                row.get::<_, String>(0)?,
                redact(&strip_ansi(&String::from_utf8_lossy(&data))),
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    let body = match format.as_str() {
        "json" => serde_json::to_string_pretty(&serde_json::json!({"sessionId":session_id,"agentId":header.0,"workingDirectory":header.1,"startedAt":header.2,"events":events})).map_err(|error| error.to_string())?,
        "md" => { let mut value = format!("# Sessão {}\n\n- Agente: `{}`\n- Diretório: `{}`\n- Início: `{}`\n\n",session_id,header.0,header.1,header.2); for (direction,text,time) in &events { value.push_str(&format!("## {direction} · {time}\n\n```text\n{text}\n```\n\n")); } value },
        _ => events.iter().map(|(direction,text,time)| format!("[{time}] {direction}\n{text}\n")).collect::<Vec<_>>().join("\n"),
    };
    let root = repository.export_root();
    fs::create_dir_all(&root).map_err(io_error)?;
    let path = root.join(format!(
        "session-{}-{}.{}",
        &session_id[..session_id.len().min(8)],
        timestamp(),
        format
    ));
    fs::write(&path, body).map_err(io_error)?;
    Ok(ExportResult {
        path: path.to_string_lossy().into_owned(),
        format,
        redaction_applied: true,
    })
}

fn redact(text: &str) -> String {
    text.lines()
        .map(|line| {
            let lower = line.to_lowercase();
            if ["api_key", "apikey", "password=", "token=", "secret="]
                .iter()
                .any(|pattern| lower.contains(pattern))
            {
                "[REDACTED: possível segredo]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn strip_ansi(text: &str) -> String {
    let mut output = String::new();
    let mut escape = false;
    for character in text.chars() {
        if character == '\u{1b}' {
            escape = true;
            continue;
        }
        if escape {
            if character.is_ascii_alphabetic() {
                escape = false;
            }
            continue;
        }
        if character != '\r' {
            output.push(character);
        }
    }
    output
}
fn timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
fn db_error(error: rusqlite::Error) -> String {
    format!("falha no banco local: {error}")
}
fn io_error(error: std::io::Error) -> String {
    format!("falha ao exportar sessão: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strips_ansi_and_redacts_common_assignments() {
        assert_eq!(strip_ansi("\u{1b}[31mhello\u{1b}[0m\r\n"), "hello\n");
        assert!(redact("TOKEN=abc").contains("REDACTED"));
    }
}
