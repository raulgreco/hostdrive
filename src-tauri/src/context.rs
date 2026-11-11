use crate::projects::ProjectRepository;
use serde::Serialize;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::State;

/// Quantas conversas recentes entram no contexto. Metadata apenas — nunca
/// bytes capturados do terminal (podem conter segredos colados na sessão).
const RECENT_SESSIONS_IN_CONTEXT: usize = 10;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectContext {
    pub project_id: String,
    pub project_name: String,
    pub generated_at: i64,
    pub markdown: String,
    pub memory_sections: usize,
    pub open_tasks: usize,
    pub recent_sessions: usize,
    /// Caminho do arquivo escrito no projeto (None quando o recurso está
    /// desligado nas configurações).
    pub file_path: Option<String>,
}

struct MemoryRow {
    section: String,
    content: String,
}

struct TaskRow {
    title: String,
    provenance: String,
}

struct SessionRow {
    agent_id: String,
    status: String,
    started_at: i64,
    resumed: bool,
}

/// Flag de injeção validada nesta máquina em 2026-11-10 via `--help` do
/// binário instalado. Só o Claude Code expõe uma opção segura para sessão
/// interativa (`--append-system-prompt`); as demais CLIs ou não têm flag de
/// contexto para a TUI (codex, opencode, kimi) ou a oferecem só no modo não
/// interativo. Nunca alegar suporte sem esta validação (NFR-007).
pub(crate) fn context_injection_flag(agent_id: &str) -> Option<&'static str> {
    match agent_id {
        "claude" => Some("--append-system-prompt"),
        _ => None,
    }
}

/// Monta o contexto agregado do projeto: memória + tarefas em aberto +
/// metadata das conversas recentes. É a ponte universal entre conversas —
/// independente de qual agente as teve.
pub(crate) fn build_project_context(
    repository: &ProjectRepository,
    project_id: &str,
) -> Result<ProjectContext, String> {
    let project = repository
        .project_by_id(project_id)
        .map_err(|error| format!("falha ao localizar projeto: {error}"))?;
    let connection = repository
        .open_for_workspace()
        .map_err(|error| format!("falha ao abrir o banco local: {error}"))?;

    let mut memory_statement = connection
        .prepare(
            "SELECT section,content FROM project_memory WHERE project_id=?1 ORDER BY section",
        )
        .map_err(|error| error.to_string())?;
    let memory_rows = memory_statement
        .query_map([project_id], |row| {
            Ok(MemoryRow {
                section: row.get(0)?,
                content: row.get(1)?,
            })
        })
        .map_err(|error| error.to_string())?;
    let memory = memory_rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    let mut task_statement = connection
        .prepare(
            "SELECT title,provenance FROM project_tasks WHERE project_id=?1 AND completed=0 ORDER BY created_at",
        )
        .map_err(|error| error.to_string())?;
    let task_rows = task_statement
        .query_map([project_id], |row| {
            Ok(TaskRow {
                title: row.get(0)?,
                provenance: row.get(1)?,
            })
        })
        .map_err(|error| error.to_string())?;
    let tasks = task_rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    let mut session_statement = connection
        .prepare(
            "SELECT s.agent_id,s.status,s.started_at,s.resumed
             FROM terminal_sessions s
             WHERE s.project_id=?1
             ORDER BY s.started_at DESC
             LIMIT ?2",
        )
        .map_err(|error| error.to_string())?;
    let session_rows = session_statement
        .query_map(rusqlite::params![project_id, RECENT_SESSIONS_IN_CONTEXT as i64], |row| {
            Ok(SessionRow {
                agent_id: row.get(0)?,
                status: row.get(1)?,
                started_at: row.get(2)?,
                resumed: row.get(3)?,
            })
        })
        .map_err(|error| error.to_string())?;
    let sessions = session_rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    let generated_at = now();
    let markdown = render_markdown(
        &project.name,
        generated_at,
        &memory,
        &tasks,
        &sessions,
    );
    Ok(ProjectContext {
        project_id: project_id.to_string(),
        project_name: project.name,
        generated_at,
        memory_sections: memory.len(),
        open_tasks: tasks.len(),
        recent_sessions: sessions.len(),
        markdown,
        file_path: None,
    })
}

/// Escreve (ou reescreve) `.eureka42/contexto.md` dentro do diretório do
/// projeto, atomicamente. Retorna None quando o usuário desligou o recurso.
pub(crate) fn write_context_file(
    repository: &ProjectRepository,
    project_id: &str,
) -> Result<Option<PathBuf>, String> {
    if !context_carryover_enabled(repository)? {
        return Ok(None);
    }
    let context = build_project_context(repository, project_id)?;
    let project_path = repository
        .project_path(project_id)
        .map_err(|error| format!("falha ao localizar projeto: {error}"))?;
    let directory = PathBuf::from(&project_path).join(".eureka42");
    fs::create_dir_all(&directory).map_err(|error| format!("falha ao criar .eureka42: {error}"))?;
    let destination = directory.join("contexto.md");
    let temporary = directory.join(".contexto.md.tmp");
    fs::write(&temporary, &context.markdown).map_err(|error| format!("falha ao gravar contexto: {error}"))?;
    fs::rename(&temporary, &destination)
        .map_err(|error| format!("falha ao publicar contexto: {error}"))?;
    Ok(Some(destination))
}

/// Reconstrói o contexto e o publica no projeto. Usado pelos pontos de
/// atualização (nova sessão, fim de sessão, mudança em memória/tarefas).
pub(crate) fn refresh_project_context(
    repository: &ProjectRepository,
    project_id: &str,
) -> Result<ProjectContext, String> {
    let mut context = build_project_context(repository, project_id)?;
    context.file_path = write_context_file(repository, project_id)?
        .map(|path| path.to_string_lossy().into_owned());
    Ok(context)
}

fn context_carryover_enabled(repository: &ProjectRepository) -> Result<bool, String> {
    repository
        .context_carryover_enabled()
        .map_err(|error| format!("falha ao ler configuração: {error}"))
}

fn render_markdown(
    project_name: &str,
    generated_at: i64,
    memory: &[MemoryRow],
    tasks: &[TaskRow],
    sessions: &[SessionRow],
) -> String {
    let mut out = String::with_capacity(4096);
    out.push_str(&format!("# Contexto do projeto — {project_name}\n\n"));
    out.push_str(&format!(
        "> Gerado pelo HostDrive em {}. Resumo para continuidade entre conversas: verifique arquivos e Git antes de agir — este texto não substitui o estado real do projeto.\n\n",
        format_timestamp(generated_at)
    ));

    out.push_str("## Memória do projeto\n\n");
    if memory.is_empty() {
        out.push_str("_(nenhuma memória registrada)_\n\n");
    } else {
        for row in memory {
            out.push_str(&format!("### {}\n\n{}\n\n", row.section, row.content.trim()));
        }
    }

    out.push_str("## Tarefas em aberto\n\n");
    if tasks.is_empty() {
        out.push_str("_(nenhuma tarefa em aberto)_\n\n");
    } else {
        for task in tasks {
            let origin = if task.provenance == "USER_CONFIRMED" {
                ""
            } else {
                " _(do agente, aguardando confirmação)_"
            };
            out.push_str(&format!("- [ ] {}{}\n", task.title, origin));
        }
        out.push('\n');
    }

    out.push_str("## Conversas recentes neste projeto\n\n");
    if sessions.is_empty() {
        out.push_str("_(nenhuma conversa registrada ainda)_\n\n");
    } else {
        for session in sessions {
            let resumed = if session.resumed { " · retomada" } else { "" };
            out.push_str(&format!(
                "- {} · {} · {}{}\n",
                format_timestamp(session.started_at),
                session.agent_id,
                session.status,
                resumed
            ));
        }
        out.push('\n');
    }

    out.push_str("## Como usar este contexto\n\n");
    out.push_str("- Ao iniciar uma **nova sessão** pelo HostDrive, o Claude Code recebe este conteúdo automaticamente (flag validada `--append-system-prompt`).\n");
    out.push_str("- Nos demais agentes, aponte para `.eureka42/contexto.md` neste diretório ou cole o resumo no início da conversa.\n");
    out.push_str("- Para repetir a última conversa nativa de um agente, use o botão **Retomar** do HostDrive.\n");
    out
}

/// Conversão civil de dias desde época para (ano, mês, dia) — algoritmo de
/// Howard Hinnant, sem dependências.
fn format_timestamp(unix: i64) -> String {
    let days = unix.div_euclid(86_400);
    let seconds = unix.rem_euclid(86_400);
    let hour = seconds / 3600;
    let minute = (seconds % 3600) / 60;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}")
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[tauri::command]
pub fn get_project_context(
    repository: State<'_, ProjectRepository>,
    project_id: String,
) -> Result<ProjectContext, String> {
    build_project_context(&repository, &project_id)
}

#[tauri::command]
pub fn refresh_project_context_command(
    repository: State<'_, ProjectRepository>,
    project_id: String,
) -> Result<ProjectContext, String> {
    refresh_project_context(&repository, &project_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository_with_project() -> (tempfile::TempDir, ProjectRepository, String) {
        let directory = tempfile::tempdir().unwrap();
        let repository = ProjectRepository::new(directory.path().join("test.sqlite"));
        repository.initialize().unwrap();
        let workspace = directory.path().join("projeto-x");
        std::fs::create_dir(&workspace).unwrap();
        let project_id = "projeto-x-id".to_string();
        repository
            .open_for_workspace()
            .unwrap()
            .execute(
                "INSERT INTO projects(id,name,path,created_at,last_opened_at) VALUES(?1,'projeto-x',?2,0,0)",
                rusqlite::params![project_id, workspace.to_string_lossy()],
            )
            .unwrap();
        (directory, repository, project_id)
    }

    #[test]
    fn context_aggregates_memory_open_tasks_and_recent_sessions() {
        let (_directory, repository, project_id) = repository_with_project();
        repository
            .open_for_workspace()
            .unwrap()
            .execute(
                "INSERT INTO project_memory(project_id,section,content,provenance,updated_at) VALUES(?1,'DECISIONS','SQLite local-first', 'USER_CONFIRMED',0)",
                rusqlite::params![project_id],
            )
            .unwrap();
        repository
            .record_session_started("ctx-1", &project_id, "codex", "/tmp", false)
            .unwrap();
        repository
            .record_session_started("ctx-2", &project_id, "claude", "/tmp", true)
            .unwrap();

        let context = build_project_context(&repository, &project_id).unwrap();
        assert_eq!(context.memory_sections, 1);
        assert_eq!(context.recent_sessions, 2);
        assert!(context.markdown.contains("# Contexto do projeto — projeto-x"));
        assert!(context.markdown.contains("SQLite local-first"));
        assert!(context.markdown.contains("claude"));
        assert!(context.markdown.contains("retomada"));
        assert!(context.markdown.contains("--append-system-prompt"));
    }

    #[test]
    fn write_context_file_publishes_markdown_inside_project_and_respects_toggle() {
        let (directory, repository, project_id) = repository_with_project();
        let written = write_context_file(&repository, &project_id).unwrap();
        let path = written.expect("carryover ligado por padrão");
        assert!(path.ends_with(".eureka42/contexto.md"));
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("# Contexto do projeto"));

        repository
            .set_context_carryover_enabled(false)
            .unwrap();
        assert!(write_context_file(&repository, &project_id)
            .unwrap()
            .is_none());
        // Arquivo anterior permanece (o app não apaga o que publicou ao
        // desligar — só para de atualizar).
        assert!(path.exists());
        drop(directory);
    }

    #[test]
    fn injection_flag_only_for_validated_agents() {
        assert_eq!(context_injection_flag("claude"), Some("--append-system-prompt"));
        assert_eq!(context_injection_flag("codex"), None);
        assert_eq!(context_injection_flag("kimi"), None);
        assert_eq!(context_injection_flag("opencode"), None);
        assert_eq!(context_injection_flag("qualquer-outro"), None);
    }

    #[test]
    fn format_timestamp_renders_civil_date() {
        // 2026-11-10 21:35:00 UTC
        assert_eq!(format_timestamp(1_794_346_500), "2026-11-10 21:35");
        assert_eq!(format_timestamp(0), "1970-01-01 00:00");
    }
}
