use crate::agents::{find_agent_executable, resume_args};
use crate::projects::ProjectRepository;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::{
    collections::HashMap,
    env,
    io::{Read, Write},
    path::PathBuf,
    sync::Mutex,
    thread,
};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

pub struct TerminalSession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

#[derive(Default)]
pub struct TerminalManager {
    sessions: Mutex<HashMap<String, TerminalSession>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalStarted {
    session_id: String,
    agent_id: String,
    working_directory: String,
    resumed: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalOutput {
    session_id: String,
    data: Vec<u8>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalExited {
    session_id: String,
}

#[tauri::command]
pub fn launch_terminal(
    app: AppHandle,
    manager: State<'_, TerminalManager>,
    repository: State<'_, ProjectRepository>,
    agent_id: String,
    working_directory: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
    resume: Option<bool>,
) -> Result<TerminalStarted, String> {
    let executable = find_agent_executable(&agent_id)
        .ok_or_else(|| format!("agente não instalado ou não permitido: {agent_id}"))?;
    let resumed = resume.unwrap_or(false);
    let resume_flags = if resumed {
        Some(
            resume_args(&agent_id)
                .ok_or_else(|| format!("retomada de sessão não suportada para {agent_id}"))?,
        )
    } else {
        None
    };
    let cwd = canonical_working_directory(working_directory)?;
    let pair = native_pty_system()
        .openpty(checked_size(cols, rows)?)
        .map_err(|error| format!("falha ao abrir PTY: {error}"))?;
    let mut command = CommandBuilder::new(executable);
    if let Some(flags) = resume_flags {
        command.args(flags);
    }
    command.cwd(&cwd);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("falha ao iniciar agente: {error}"))?;
    drop(pair.slave);
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("falha ao abrir saída do PTY: {error}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| format!("falha ao abrir entrada do PTY: {error}"))?;
    let session_id = Uuid::new_v4().to_string();
    let output_session_id = session_id.clone();
    let cwd_string = cwd.to_string_lossy().into_owned();
    let project_id = repository
        .project_id_for_path(&cwd_string)
        .map_err(|error| format!("falha ao localizar projeto: {error}"))?
        .ok_or_else(|| "o diretório não pertence a um projeto ativo".to_string())?;
    if let Err(error) = repository.record_session_started(
        &session_id,
        &project_id,
        &agent_id,
        &cwd_string,
        resumed,
    ) {
        let _ = child.kill();
        return Err(format!("falha ao registrar sessão: {error}"));
    }
    let output_repository = repository.inner().clone();

    thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(length) => {
                    if output_repository
                        .should_capture_terminal_events()
                        .unwrap_or(false)
                    {
                        let _ = output_repository.record_terminal_event(
                            &output_session_id,
                            "OUTPUT",
                            &buffer[..length],
                        );
                    }
                    let _ = app.emit(
                        "terminal-output",
                        TerminalOutput {
                            session_id: output_session_id.clone(),
                            data: buffer[..length].to_vec(),
                        },
                    );
                }
            }
        }
        let _ = output_repository.mark_session_ended(&output_session_id, "EXITED");
        let _ = app.emit(
            "terminal-exited",
            TerminalExited {
                session_id: output_session_id,
            },
        );
    });

    manager
        .sessions
        .lock()
        .map_err(|_| "gerenciador de terminal indisponível".to_string())?
        .insert(
            session_id.clone(),
            TerminalSession {
                master: pair.master,
                writer,
                child,
            },
        );
    Ok(TerminalStarted {
        session_id,
        agent_id,
        working_directory: cwd_string,
        resumed,
    })
}

#[tauri::command]
pub fn send_terminal_input(
    manager: State<'_, TerminalManager>,
    repository: State<'_, ProjectRepository>,
    session_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let mut sessions = manager
        .sessions
        .lock()
        .map_err(|_| "gerenciador de terminal indisponível".to_string())?;
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| "sessão de terminal não encontrada".to_string())?;
    session
        .writer
        .write_all(&data)
        .and_then(|_| session.writer.flush())
        .map_err(|error| format!("falha ao escrever no PTY: {error}"))?;
    if repository
        .should_capture_terminal_events()
        .map_err(|error| format!("falha ao ler configuração de captura: {error}"))?
    {
        repository
            .record_terminal_event(&session_id, "INPUT", &data)
            .map_err(|error| format!("falha ao registrar entrada: {error}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn resize_terminal(
    manager: State<'_, TerminalManager>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let size = checked_size(Some(cols), Some(rows))?;
    let sessions = manager
        .sessions
        .lock()
        .map_err(|_| "gerenciador de terminal indisponível".to_string())?;
    sessions
        .get(&session_id)
        .ok_or_else(|| "sessão de terminal não encontrada".to_string())?
        .master
        .resize(size)
        .map_err(|error| format!("falha ao redimensionar PTY: {error}"))
}

#[tauri::command]
pub fn close_terminal(
    manager: State<'_, TerminalManager>,
    repository: State<'_, ProjectRepository>,
    session_id: String,
) -> Result<(), String> {
    let mut session = manager
        .sessions
        .lock()
        .map_err(|_| "gerenciador de terminal indisponível".to_string())?
        .remove(&session_id)
        .ok_or_else(|| "sessão de terminal não encontrada".to_string())?;
    session
        .child
        .kill()
        .map_err(|error| format!("falha ao encerrar agente: {error}"))?;
    repository
        .mark_session_ended(&session_id, "TERMINATED")
        .map_err(|error| format!("falha ao finalizar registro: {error}"))
}

fn canonical_working_directory(requested: Option<String>) -> Result<PathBuf, String> {
    let path = match requested {
        Some(value) => PathBuf::from(value),
        None => env::current_dir().map_err(|error| format!("cwd indisponível: {error}"))?,
    };
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("diretório inválido: {error}"))?;
    if !canonical.is_dir() {
        return Err("o diretório de trabalho não é uma pasta".into());
    }
    Ok(canonical)
}

fn checked_size(cols: Option<u16>, rows: Option<u16>) -> Result<PtySize, String> {
    let cols = cols.unwrap_or(120);
    let rows = rows.unwrap_or(32);
    if !(2..=1000).contains(&cols) || !(2..=500).contains(&rows) {
        return Err("dimensões de terminal fora do intervalo permitido".into());
    }
    Ok(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_terminal_dimensions() {
        assert!(checked_size(Some(1), Some(24)).is_err());
        assert!(checked_size(Some(80), Some(501)).is_err());
        assert!(checked_size(Some(80), Some(24)).is_ok());
    }

    #[test]
    fn rejects_missing_working_directory() {
        assert!(canonical_working_directory(Some("/definitely/missing/uai-cli".into())).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn unix_pty_captures_real_process_output() {
        let pair = native_pty_system()
            .openpty(checked_size(Some(80), Some(24)).unwrap())
            .unwrap();
        let mut command = CommandBuilder::new("/bin/sh");
        command.args(["-c", "printf 'PTY_OK\\n'"]);
        let mut child = pair.slave.spawn_command(command).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let mut output = String::new();
        reader.read_to_string(&mut output).unwrap();
        let status = child.wait().unwrap();
        assert!(status.success());
        assert!(output.contains("PTY_OK"));
    }
}
