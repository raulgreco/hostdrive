use crate::projects::ProjectRepository;
use rusqlite::params;
use serde::Serialize;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEntry {
    section: String,
    content: String,
    provenance: String,
    updated_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTask {
    id: String,
    title: String,
    completed: bool,
    provenance: String,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    available: bool,
    branch: Option<String>,
    head: Option<String>,
    dirty: bool,
    changes: Vec<String>,
    error: Option<String>,
}

#[tauri::command]
pub fn list_project_memory(
    repository: State<'_, ProjectRepository>,
    project_id: String,
) -> Result<Vec<MemoryEntry>, String> {
    let connection = repository.open_for_workspace().map_err(db_error)?;
    let mut statement = connection.prepare("SELECT section,content,provenance,updated_at FROM project_memory WHERE project_id=?1 ORDER BY section").map_err(db_error)?;
    let entries = statement
        .query_map([project_id], |row| {
            Ok(MemoryEntry {
                section: row.get(0)?,
                content: row.get(1)?,
                provenance: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(entries)
}

#[tauri::command]
pub fn save_project_memory(
    repository: State<'_, ProjectRepository>,
    project_id: String,
    section: String,
    content: String,
) -> Result<MemoryEntry, String> {
    const SECTIONS: [&str; 5] = [
        "IDENTITY",
        "REQUIREMENTS",
        "ARCHITECTURE",
        "DECISIONS",
        "CURRENT_STATE",
    ];
    if !SECTIONS.contains(&section.as_str()) {
        return Err("seção de memória inválida".into());
    }
    if content.len() > 200_000 {
        return Err("seção de memória excede 200 KB".into());
    }
    repository.project_path(&project_id).map_err(db_error)?;
    let now = timestamp();
    repository.open_for_workspace().map_err(db_error)?.execute(
        "INSERT INTO project_memory(project_id,section,content,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(project_id,section) DO UPDATE SET content=excluded.content,provenance='USER_CONFIRMED',updated_at=excluded.updated_at",
        params![project_id, section, content, now],
    ).map_err(db_error)?;
    Ok(MemoryEntry {
        section,
        content,
        provenance: "USER_CONFIRMED".into(),
        updated_at: now,
    })
}

#[tauri::command]
pub fn list_project_tasks(
    repository: State<'_, ProjectRepository>,
    project_id: String,
) -> Result<Vec<ProjectTask>, String> {
    let connection = repository.open_for_workspace().map_err(db_error)?;
    let mut statement = connection.prepare("SELECT id,title,completed,provenance,created_at,updated_at FROM project_tasks WHERE project_id=?1 ORDER BY completed,created_at DESC").map_err(db_error)?;
    let tasks = statement
        .query_map([project_id], |row| {
            Ok(ProjectTask {
                id: row.get(0)?,
                title: row.get(1)?,
                completed: row.get(2)?,
                provenance: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(db_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_error)?;
    Ok(tasks)
}

#[tauri::command]
pub fn add_project_task(
    repository: State<'_, ProjectRepository>,
    project_id: String,
    title: String,
) -> Result<ProjectTask, String> {
    let title = title.trim();
    if title.is_empty() || title.len() > 500 {
        return Err("tarefa deve ter entre 1 e 500 caracteres".into());
    }
    repository.project_path(&project_id).map_err(db_error)?;
    let task = ProjectTask {
        id: Uuid::new_v4().to_string(),
        title: title.into(),
        completed: false,
        provenance: "USER_CONFIRMED".into(),
        created_at: timestamp(),
        updated_at: timestamp(),
    };
    repository.open_for_workspace().map_err(db_error)?.execute("INSERT INTO project_tasks(id,project_id,title,completed,provenance,created_at,updated_at) VALUES(?1,?2,?3,0,?4,?5,?6)", params![task.id,project_id,task.title,task.provenance,task.created_at,task.updated_at]).map_err(db_error)?;
    Ok(task)
}

#[tauri::command]
pub fn set_project_task_completed(
    repository: State<'_, ProjectRepository>,
    task_id: String,
    completed: bool,
) -> Result<(), String> {
    let changed = repository
        .open_for_workspace()
        .map_err(db_error)?
        .execute(
            "UPDATE project_tasks SET completed=?1,updated_at=?2 WHERE id=?3",
            params![completed, timestamp(), task_id],
        )
        .map_err(db_error)?;
    if changed == 0 {
        return Err("tarefa não encontrada".into());
    }
    Ok(())
}

#[tauri::command]
pub fn get_git_status(
    repository: State<'_, ProjectRepository>,
    project_id: String,
) -> Result<GitStatus, String> {
    let path = repository.project_path(&project_id).map_err(db_error)?;
    if git(&path, &["rev-parse", "--is-inside-work-tree"]).as_deref() != Ok("true") {
        return Ok(GitStatus {
            available: false,
            branch: None,
            head: None,
            dirty: false,
            changes: vec![],
            error: Some("Este projeto não é um repositório Git.".into()),
        });
    }
    let branch = git(&path, &["branch", "--show-current"])
        .ok()
        .filter(|value| !value.is_empty());
    let head = git(&path, &["rev-parse", "--short", "HEAD"]).ok();
    let changes = git(
        &path,
        &["status", "--porcelain=v1", "--untracked-files=normal"],
    )
    .unwrap_or_default()
    .lines()
    .map(str::to_string)
    .collect::<Vec<_>>();
    Ok(GitStatus {
        available: true,
        branch,
        head,
        dirty: !changes.is_empty(),
        changes,
        error: None,
    })
}

fn git(path: &str, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(arguments)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
