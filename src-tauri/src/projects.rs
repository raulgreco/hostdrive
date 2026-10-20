use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::State;
use uuid::Uuid;

#[derive(Clone)]
pub struct ProjectRepository {
    database_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) path: String,
    is_favorite: bool,
    is_archived: bool,
    created_at: i64,
    last_opened_at: i64,
    /// Identificador estável usado como nome do objeto remoto no Nextcloud —
    /// independente do caminho local, que muda de máquina para máquina.
    /// `None` até a primeira sincronização.
    pub(crate) nextcloud_sync_id: Option<String>,
    pub(crate) last_synced_at: i64,
    /// Opt-in explícito por projeto — nunca ligado por padrão (ver
    /// docs/decisions/005-nextcloud-sync.md): histórico de terminal pode
    /// conter segredos colados durante a sessão.
    pub(crate) sync_terminal_history: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    id: String,
    project_id: String,
    agent_id: String,
    working_directory: String,
    status: String,
    started_at: i64,
    ended_at: Option<i64>,
    event_count: i64,
    captured_bytes: i64,
    /// REQ-007 — `true` quando a sessão foi iniciada retomando o histórico
    /// nativo do agente (ex.: `claude --continue`) em vez de uma sessão nova.
    resumed: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub(crate) capture_terminal_events: bool,
    pub(crate) retention_days: i64,
    /// NFR-006 — preferência de tema: "system" segue o SO, ou "light"/"dark"
    /// força a escolha manual.
    pub(crate) theme: String,
    /// Configuração de sincronização opcional via Nextcloud — ver
    /// docs/decisions/005-nextcloud-sync.md. `nextcloud_url`/`username` não
    /// são segredo e vivem aqui; a senha de app e a frase de criptografia
    /// ficam só no chaveiro do sistema operacional (nunca no SQLite).
    pub(crate) nextcloud_url: String,
    pub(crate) nextcloud_username: String,
    pub(crate) nextcloud_sync_enabled: bool,
}

impl ProjectRepository {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }

    pub fn initialize(&self) -> Result<(), rusqlite::Error> {
        let connection = self.open()?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS projects (
               id TEXT PRIMARY KEY NOT NULL,
               name TEXT NOT NULL,
               path TEXT NOT NULL UNIQUE,
               is_favorite INTEGER NOT NULL DEFAULT 0,
               is_archived INTEGER NOT NULL DEFAULT 0,
               created_at INTEGER NOT NULL,
               last_opened_at INTEGER NOT NULL,
               nextcloud_sync_id TEXT,
               last_synced_at INTEGER NOT NULL DEFAULT 0,
               sync_terminal_history INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS idx_projects_recent ON projects(is_archived, last_opened_at DESC);
             CREATE TABLE IF NOT EXISTS terminal_sessions (
               id TEXT PRIMARY KEY NOT NULL,
               project_id TEXT NOT NULL REFERENCES projects(id),
               agent_id TEXT NOT NULL,
               working_directory TEXT NOT NULL,
               status TEXT NOT NULL,
               started_at INTEGER NOT NULL,
               ended_at INTEGER,
               resumed INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS idx_terminal_sessions_project ON terminal_sessions(project_id, started_at DESC);
             CREATE TABLE IF NOT EXISTS terminal_events (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE CASCADE,
               direction TEXT NOT NULL,
               data BLOB NOT NULL,
               created_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_terminal_events_session ON terminal_events(session_id, id);
             CREATE VIRTUAL TABLE IF NOT EXISTS terminal_event_search USING fts5(
               session_id UNINDEXED,
               direction UNINDEXED,
               content,
               tokenize='unicode61'
             );
             INSERT INTO terminal_event_search(rowid,session_id,direction,content)
             SELECT e.id,e.session_id,e.direction,CAST(e.data AS TEXT)
             FROM terminal_events e
             WHERE NOT EXISTS (SELECT 1 FROM terminal_event_search f WHERE f.rowid=e.id);"
        )?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS app_settings (
               id INTEGER PRIMARY KEY CHECK (id=1),
               capture_terminal_events INTEGER NOT NULL DEFAULT 1,
               retention_days INTEGER NOT NULL DEFAULT 90,
               theme TEXT NOT NULL DEFAULT 'dark',
               nextcloud_url TEXT NOT NULL DEFAULT '',
               nextcloud_username TEXT NOT NULL DEFAULT '',
               nextcloud_sync_enabled INTEGER NOT NULL DEFAULT 0
             );
             INSERT OR IGNORE INTO app_settings(id) VALUES(1);",
        )?;
        // Migração para bancos criados antes destas colunas existirem.
        // SQLite não suporta `ADD COLUMN IF NOT EXISTS`; ignoramos o erro
        // "duplicate column name" quando a coluna já existe.
        ensure_column(&connection, "terminal_sessions", "resumed", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(&connection, "app_settings", "theme", "TEXT NOT NULL DEFAULT 'dark'")?;
        ensure_column(&connection, "app_settings", "nextcloud_url", "TEXT NOT NULL DEFAULT ''")?;
        ensure_column(&connection, "app_settings", "nextcloud_username", "TEXT NOT NULL DEFAULT ''")?;
        ensure_column(&connection, "app_settings", "nextcloud_sync_enabled", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(&connection, "projects", "nextcloud_sync_id", "TEXT")?;
        ensure_column(&connection, "projects", "last_synced_at", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(&connection, "projects", "sync_terminal_history", "INTEGER NOT NULL DEFAULT 0")?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS project_memory (
               project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
               section TEXT NOT NULL,
               content TEXT NOT NULL DEFAULT '',
               provenance TEXT NOT NULL DEFAULT 'USER_CONFIRMED',
               updated_at INTEGER NOT NULL,
               PRIMARY KEY(project_id,section)
             );
             CREATE TABLE IF NOT EXISTS project_tasks (
               id TEXT PRIMARY KEY NOT NULL,
               project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
               title TEXT NOT NULL,
               completed INTEGER NOT NULL DEFAULT 0,
               provenance TEXT NOT NULL DEFAULT 'USER_CONFIRMED',
               created_at INTEGER NOT NULL,
               updated_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_project_tasks_project ON project_tasks(project_id,completed,created_at DESC);",
        )?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS handoffs (
               id TEXT PRIMARY KEY NOT NULL,
               project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
               source_agent TEXT NOT NULL,
               target_agent TEXT NOT NULL,
               status TEXT NOT NULL,
               package_path TEXT NOT NULL,
               created_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_handoffs_project ON handoffs(project_id,created_at DESC);",
        )?;
        connection.execute(
            "UPDATE terminal_sessions SET status='INTERRUPTED',ended_at=?1 WHERE status='RUNNING'",
            [unix_timestamp().unwrap_or_default()],
        )?;
        Ok(())
    }

    fn open(&self) -> Result<Connection, rusqlite::Error> {
        Connection::open(&self.database_path)
    }

    pub(crate) fn open_for_workspace(&self) -> Result<Connection, rusqlite::Error> {
        self.open()
    }

    pub(crate) fn handoff_root(&self) -> PathBuf {
        self.database_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("handoffs")
    }

    pub(crate) fn export_root(&self) -> PathBuf {
        self.database_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("exports")
    }

    pub(crate) fn project_id_for_path(
        &self,
        path: &str,
    ) -> Result<Option<String>, rusqlite::Error> {
        self.open()?
            .query_row(
                "SELECT id FROM projects WHERE path=?1 AND is_archived=0",
                [path],
                |row| row.get(0),
            )
            .optional()
    }

    pub(crate) fn project_path(&self, project_id: &str) -> Result<String, rusqlite::Error> {
        self.open()?.query_row(
            "SELECT path FROM projects WHERE id=?1 AND is_archived=0",
            [project_id],
            |row| row.get(0),
        )
    }

    /// Carrega o registro completo do projeto — usado pela sincronização
    /// (`sync.rs`) para ler nome, `nextcloud_sync_id` e `last_synced_at`.
    pub(crate) fn project_by_id(&self, project_id: &str) -> Result<Project, rusqlite::Error> {
        self.open()?.query_row(
            &format!("SELECT {PROJECT_COLUMNS} FROM projects WHERE id=?1"),
            [project_id],
            project_from_row,
        )
    }

    /// Grava o identificador remoto (gerado na primeira sincronização) e o
    /// horário da última sincronização bem-sucedida deste projeto.
    pub(crate) fn mark_project_synced(
        &self,
        project_id: &str,
        nextcloud_sync_id: &str,
        synced_at: i64,
    ) -> Result<(), rusqlite::Error> {
        self.open()?.execute(
            "UPDATE projects SET nextcloud_sync_id=?1,last_synced_at=?2 WHERE id=?3",
            params![nextcloud_sync_id, synced_at, project_id],
        )?;
        Ok(())
    }

    /// REQ opt-in — liga/desliga a inclusão do histórico de terminal deste
    /// projeto específico nos pacotes sincronizados. Nunca ligado por padrão.
    pub(crate) fn set_project_terminal_sync(
        &self,
        project_id: &str,
        enabled: bool,
    ) -> Result<(), rusqlite::Error> {
        let changed = self.open()?.execute(
            "UPDATE projects SET sync_terminal_history=?1 WHERE id=?2",
            params![enabled, project_id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub(crate) fn record_session_started(
        &self,
        id: &str,
        project_id: &str,
        agent_id: &str,
        cwd: &str,
        resumed: bool,
    ) -> Result<(), rusqlite::Error> {
        self.open()?.execute("INSERT INTO terminal_sessions(id,project_id,agent_id,working_directory,status,started_at,resumed) VALUES(?1,?2,?3,?4,'RUNNING',?5,?6)", params![id, project_id, agent_id, cwd, unix_timestamp().unwrap_or_default(), resumed])?;
        Ok(())
    }

    pub(crate) fn record_terminal_event(
        &self,
        session_id: &str,
        direction: &str,
        data: &[u8],
    ) -> Result<(), rusqlite::Error> {
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO terminal_events(session_id,direction,data,created_at) VALUES(?1,?2,?3,?4)",
            params![
                session_id,
                direction,
                data,
                unix_timestamp().unwrap_or_default()
            ],
        )?;
        let event_id = transaction.last_insert_rowid();
        transaction.execute(
            "INSERT INTO terminal_event_search(rowid,session_id,direction,content) VALUES(?1,?2,?3,?4)",
            params![event_id, session_id, direction, String::from_utf8_lossy(data)],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn mark_session_ended(
        &self,
        session_id: &str,
        status: &str,
    ) -> Result<(), rusqlite::Error> {
        self.open()?.execute(
            "UPDATE terminal_sessions SET status=?1,ended_at=?2 WHERE id=?3",
            params![status, unix_timestamp().unwrap_or_default(), session_id],
        )?;
        Ok(())
    }

    pub(crate) fn should_capture_terminal_events(&self) -> Result<bool, rusqlite::Error> {
        self.open()?.query_row(
            "SELECT capture_terminal_events FROM app_settings WHERE id=1",
            [],
            |row| row.get(0),
        )
    }
}

/// Adiciona `column` a `table` quando ainda não existir. Necessário porque
/// SQLite não aceita `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`.
fn ensure_column(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), rusqlite::Error> {
    match connection.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
        [],
    ) {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(_, Some(message)))
            if message.contains("duplicate column name") =>
        {
            Ok(())
        }
        Err(error) => Err(error),
    }
}

#[tauri::command]
pub fn list_projects(
    repository: State<'_, ProjectRepository>,
    include_archived: Option<bool>,
) -> Result<Vec<Project>, String> {
    list(&repository, include_archived.unwrap_or(false)).map_err(database_error)
}

#[tauri::command]
pub fn add_project(
    repository: State<'_, ProjectRepository>,
    path: String,
) -> Result<Project, String> {
    add(&repository, Path::new(&path)).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_project_favorite(
    repository: State<'_, ProjectRepository>,
    project_id: String,
    favorite: bool,
) -> Result<(), String> {
    update_flag(&repository, &project_id, "is_favorite", favorite).map_err(database_error)
}

#[tauri::command]
pub fn set_project_archived(
    repository: State<'_, ProjectRepository>,
    project_id: String,
    archived: bool,
) -> Result<(), String> {
    update_flag(&repository, &project_id, "is_archived", archived).map_err(database_error)
}

#[tauri::command]
pub fn set_project_terminal_sync(
    repository: State<'_, ProjectRepository>,
    project_id: String,
    enabled: bool,
) -> Result<(), String> {
    repository
        .set_project_terminal_sync(&project_id, enabled)
        .map_err(database_error)
}

#[tauri::command]
pub fn list_terminal_sessions(
    repository: State<'_, ProjectRepository>,
    project_id: String,
    limit: Option<u16>,
    offset: Option<u32>,
    agent_id: Option<String>,
    status: Option<String>,
    query: Option<String>,
    started_after: Option<i64>,
) -> Result<Vec<SessionSummary>, String> {
    list_sessions(
        &repository,
        &project_id,
        limit.unwrap_or(50),
        offset.unwrap_or(0),
        agent_id.as_deref().unwrap_or(""),
        status.as_deref().unwrap_or(""),
        query.as_deref().unwrap_or(""),
        started_after.unwrap_or(0),
    )
    .map_err(database_error)
}

#[tauri::command]
pub fn get_app_settings(repository: State<'_, ProjectRepository>) -> Result<AppSettings, String> {
    read_app_settings(&repository).map_err(database_error)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_app_settings(
    repository: State<'_, ProjectRepository>,
    capture_terminal_events: bool,
    retention_days: i64,
    theme: String,
    nextcloud_url: String,
    nextcloud_username: String,
    nextcloud_sync_enabled: bool,
) -> Result<AppSettings, String> {
    write_app_settings(
        &repository,
        capture_terminal_events,
        retention_days,
        theme,
        nextcloud_url,
        nextcloud_username,
        nextcloud_sync_enabled,
    )
    .map_err(|error| error.to_string())
}

fn list(
    repository: &ProjectRepository,
    include_archived: bool,
) -> Result<Vec<Project>, rusqlite::Error> {
    let connection = repository.open()?;
    let sql = if include_archived {
        format!("SELECT {PROJECT_COLUMNS} FROM projects ORDER BY is_favorite DESC,last_opened_at DESC")
    } else {
        format!("SELECT {PROJECT_COLUMNS} FROM projects WHERE is_archived=0 ORDER BY is_favorite DESC,last_opened_at DESC")
    };
    let sql = sql.as_str();
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map([], project_from_row)?;
    rows.collect()
}

fn list_sessions(
    repository: &ProjectRepository,
    project_id: &str,
    limit: u16,
    offset: u32,
    agent_id: &str,
    status: &str,
    query: &str,
    started_after: i64,
) -> Result<Vec<SessionSummary>, rusqlite::Error> {
    let limit = limit.clamp(1, 200);
    let connection = repository.open()?;
    let normalized_query = fts_query(query);
    let base =
        "SELECT s.id,s.project_id,s.agent_id,s.working_directory,s.status,s.started_at,s.ended_at,
                COUNT(e.id),COALESCE(SUM(length(e.data)),0),s.resumed
         FROM terminal_sessions s LEFT JOIN terminal_events e ON e.session_id=s.id
         WHERE s.project_id=?1 AND (?2='' OR s.agent_id=?2) AND (?3='' OR s.status=?3) AND (?4=0 OR s.started_at>=?4)";
    let sql = if normalized_query.is_empty() {
        format!("{base} GROUP BY s.id ORDER BY s.started_at DESC LIMIT ?5 OFFSET ?6")
    } else {
        format!("{base} AND EXISTS (SELECT 1 FROM terminal_event_search f WHERE f.session_id=s.id AND terminal_event_search MATCH ?5) GROUP BY s.id ORDER BY s.started_at DESC LIMIT ?6 OFFSET ?7")
    };
    let mut statement = connection.prepare(&sql)?;
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok(SessionSummary {
            id: row.get(0)?,
            project_id: row.get(1)?,
            agent_id: row.get(2)?,
            working_directory: row.get(3)?,
            status: row.get(4)?,
            started_at: row.get(5)?,
            ended_at: row.get(6)?,
            event_count: row.get(7)?,
            captured_bytes: row.get(8)?,
            resumed: row.get(9)?,
        })
    };
    if normalized_query.is_empty() {
        statement
            .query_map(
                params![project_id, agent_id, status, started_after, limit, offset],
                map_row,
            )?
            .collect()
    } else {
        statement
            .query_map(
                params![
                    project_id,
                    agent_id,
                    status,
                    started_after,
                    normalized_query,
                    limit,
                    offset
                ],
                map_row,
            )?
            .collect()
    }
}

fn fts_query(input: &str) -> String {
    input
        .split_whitespace()
        .filter_map(|term| {
            let clean: String = term
                .chars()
                .filter(|character| {
                    character.is_alphanumeric() || *character == '_' || *character == '-'
                })
                .collect();
            (!clean.is_empty()).then(|| format!("\"{}\"*", clean.replace('"', "\"\"")))
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

pub(crate) fn read_app_settings(repository: &ProjectRepository) -> Result<AppSettings, rusqlite::Error> {
    repository.open()?.query_row(
        "SELECT capture_terminal_events,retention_days,theme,nextcloud_url,nextcloud_username,nextcloud_sync_enabled FROM app_settings WHERE id=1",
        [],
        |row| {
            Ok(AppSettings {
                capture_terminal_events: row.get(0)?,
                retention_days: row.get(1)?,
                theme: row.get(2)?,
                nextcloud_url: row.get(3)?,
                nextcloud_username: row.get(4)?,
                nextcloud_sync_enabled: row.get(5)?,
            })
        },
    )
}

const VALID_THEMES: [&str; 3] = ["system", "light", "dark"];

#[allow(clippy::too_many_arguments)]
fn write_app_settings(
    repository: &ProjectRepository,
    capture_terminal_events: bool,
    retention_days: i64,
    theme: String,
    nextcloud_url: String,
    nextcloud_username: String,
    nextcloud_sync_enabled: bool,
) -> Result<AppSettings, Box<dyn std::error::Error>> {
    if !(1..=3650).contains(&retention_days) {
        return Err("retenção deve ficar entre 1 e 3650 dias".into());
    }
    if !VALID_THEMES.contains(&theme.as_str()) {
        return Err("tema inválido: use system, light ou dark".into());
    }
    if nextcloud_sync_enabled && (nextcloud_url.trim().is_empty() || nextcloud_username.trim().is_empty()) {
        return Err("informe o endereço do servidor e o usuário do Nextcloud antes de ativar a sincronização".into());
    }
    repository.open()?.execute(
        "UPDATE app_settings SET capture_terminal_events=?1,retention_days=?2,theme=?3,nextcloud_url=?4,nextcloud_username=?5,nextcloud_sync_enabled=?6 WHERE id=1",
        params![capture_terminal_events, retention_days, theme, nextcloud_url.trim(), nextcloud_username.trim(), nextcloud_sync_enabled],
    )?;
    Ok(read_app_settings(repository)?)
}

fn add(
    repository: &ProjectRepository,
    requested: &Path,
) -> Result<Project, Box<dyn std::error::Error>> {
    let canonical = requested.canonicalize()?;
    if !canonical.is_dir() {
        return Err("o projeto precisa ser um diretório".into());
    }
    let path = canonical.to_string_lossy().into_owned();
    let name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("Projeto")
        .to_string();
    let now = unix_timestamp()?;
    let connection = repository.open()?;
    let existing_id: Option<String> = connection
        .query_row("SELECT id FROM projects WHERE path=?1", [&path], |row| {
            row.get(0)
        })
        .optional()?;
    let id = existing_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    connection.execute(
        "INSERT INTO projects(id,name,path,created_at,last_opened_at) VALUES(?1,?2,?3,?4,?4)
         ON CONFLICT(path) DO UPDATE SET name=excluded.name,last_opened_at=excluded.last_opened_at,is_archived=0",
        params![id, name, path, now]
    )?;
    connection.query_row(
        &format!("SELECT {PROJECT_COLUMNS} FROM projects WHERE path=?1"),
        [&path], project_from_row
    ).map_err(Into::into)
}

fn update_flag(
    repository: &ProjectRepository,
    id: &str,
    column: &str,
    value: bool,
) -> Result<(), rusqlite::Error> {
    let sql = match column {
        "is_favorite" => "UPDATE projects SET is_favorite=?1 WHERE id=?2",
        "is_archived" => "UPDATE projects SET is_archived=?1 WHERE id=?2",
        _ => return Err(rusqlite::Error::InvalidParameterName(column.into())),
    };
    let changed = repository.open()?.execute(sql, params![value, id])?;
    if changed == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}

const PROJECT_COLUMNS: &str = "id,name,path,is_favorite,is_archived,created_at,last_opened_at,nextcloud_sync_id,last_synced_at,sync_terminal_history";

fn project_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        path: row.get(2)?,
        is_favorite: row.get(3)?,
        is_archived: row.get(4)?,
        created_at: row.get(5)?,
        last_opened_at: row.get(6)?,
        nextcloud_sync_id: row.get(7)?,
        last_synced_at: row.get(8)?,
        sync_terminal_history: row.get(9)?,
    })
}

fn unix_timestamp() -> Result<i64, std::time::SystemTimeError> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64)
}
fn database_error(error: rusqlite::Error) -> String {
    format!("falha no banco local: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository() -> (tempfile::TempDir, ProjectRepository) {
        let directory = tempfile::tempdir().unwrap();
        let repository = ProjectRepository::new(directory.path().join("test.sqlite"));
        repository.initialize().unwrap();
        (directory, repository)
    }

    #[test]
    fn adds_reopens_favorites_and_archives_without_touching_directory() {
        let (directory, repository) = repository();
        let workspace = directory.path().join("Meu Projeto");
        std::fs::create_dir(&workspace).unwrap();
        let first = add(&repository, &workspace).unwrap();
        let reopened = add(&repository, &workspace).unwrap();
        assert_eq!(first.id, reopened.id);
        update_flag(&repository, &first.id, "is_favorite", true).unwrap();
        assert!(list(&repository, false).unwrap()[0].is_favorite);
        update_flag(&repository, &first.id, "is_archived", true).unwrap();
        assert!(list(&repository, false).unwrap().is_empty());
        assert_eq!(list(&repository, true).unwrap().len(), 1);
        assert!(workspace.exists());
    }

    #[test]
    fn rejects_missing_project_directory() {
        let (_directory, repository) = repository();
        assert!(add(&repository, Path::new("/definitely/missing/project")).is_err());
    }

    #[test]
    fn records_and_summarizes_terminal_session_events() {
        let (directory, repository) = repository();
        let workspace = directory.path().join("workspace");
        std::fs::create_dir(&workspace).unwrap();
        let project = add(&repository, &workspace).unwrap();
        repository
            .record_session_started(
                "session-1",
                &project.id,
                "codex",
                project.path.as_str(),
                true,
            )
            .unwrap();
        repository
            .record_terminal_event("session-1", "OUTPUT", b"hello")
            .unwrap();
        repository
            .record_terminal_event("session-1", "INPUT", b"world")
            .unwrap();
        repository
            .mark_session_ended("session-1", "EXITED")
            .unwrap();
        let sessions = list_sessions(&repository, &project.id, 50, 0, "", "", "", 0).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].event_count, 2);
        assert_eq!(sessions[0].captured_bytes, 10);
        assert_eq!(sessions[0].status, "EXITED");
        assert!(sessions[0].resumed);
        assert_eq!(
            list_sessions(
                &repository,
                &project.id,
                50,
                0,
                "codex",
                "EXITED",
                "hello",
                0
            )
            .unwrap()
            .len(),
            1
        );
        assert!(
            list_sessions(&repository, &project.id, 50, 0, "claude", "", "hello", 0)
                .unwrap()
                .is_empty()
        );
        assert!(
            list_sessions(&repository, &project.id, 50, 0, "", "", "absent", 0)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn persists_local_privacy_settings() {
        let (_directory, repository) = repository();
        let initial = read_app_settings(&repository).unwrap();
        assert!(initial.capture_terminal_events);
        assert_eq!(initial.theme, "dark");
        assert!(!initial.nextcloud_sync_enabled);
        let changed = write_app_settings(
            &repository,
            false,
            30,
            "light".into(),
            String::new(),
            String::new(),
            false,
        )
        .unwrap();
        assert!(!changed.capture_terminal_events);
        assert_eq!(changed.retention_days, 30);
        assert_eq!(changed.theme, "light");
        assert!(!repository.should_capture_terminal_events().unwrap());
        assert!(write_app_settings(&repository, true, 0, "light".into(), String::new(), String::new(), false).is_err());
        assert!(write_app_settings(&repository, true, 30, "neon".into(), String::new(), String::new(), false).is_err());
        // Não pode ativar a sincronização sem URL/usuário do Nextcloud configurados.
        assert!(write_app_settings(&repository, true, 30, "dark".into(), String::new(), String::new(), true).is_err());
        let synced = write_app_settings(
            &repository,
            true,
            30,
            "dark".into(),
            "https://nextcloud.example.com".into(),
            "raul".into(),
            true,
        )
        .unwrap();
        assert!(synced.nextcloud_sync_enabled);
        assert_eq!(synced.nextcloud_url, "https://nextcloud.example.com");
    }

    /// NFR-008 — mede a busca FTS5 contra um dataset sintético realista em
    /// vez de alegar a meta sem medir. Condições publicadas: Apple M4 (Mac
    /// mini, 24 GB), SQLite WAL local, 2026-10-13. Não é uma garantia para
    /// todo hardware — é a evidência exigida por `docs/09-TESTES-E-ACEITE.md`.
    #[test]
    fn fts5_search_stays_within_budget_on_synthetic_dataset() {
        let (directory, repository) = repository();
        let workspace = directory.path().join("workspace-benchmark");
        std::fs::create_dir(&workspace).unwrap();
        let project = add(&repository, &workspace).unwrap();

        const SESSIONS: usize = 400;
        const EVENTS_PER_SESSION: usize = 12;
        for session_index in 0..SESSIONS {
            let session_id = format!("bench-session-{session_index}");
            repository
                .record_session_started(
                    &session_id,
                    &project.id,
                    if session_index % 2 == 0 { "codex" } else { "claude" },
                    project.path.as_str(),
                    false,
                )
                .unwrap();
            for event_index in 0..EVENTS_PER_SESSION {
                let text = format!(
                    "linha {event_index} da sessao {session_index}: cargo build --release artefato-{session_index}"
                );
                repository
                    .record_terminal_event(&session_id, "OUTPUT", text.as_bytes())
                    .unwrap();
            }
            repository.mark_session_ended(&session_id, "EXITED").unwrap();
        }

        let started = std::time::Instant::now();
        let results = list_sessions(&repository, &project.id, 50, 0, "", "", "artefato-137", 0).unwrap();
        let elapsed = started.elapsed();
        println!("[NFR-008] busca FTS5 em {SESSIONS} sessões / {} eventos: {elapsed:?}", SESSIONS * EVENTS_PER_SESSION);

        assert_eq!(results.len(), 1, "a busca textual deve achar a sessão específica");
        assert!(
            elapsed.as_millis() < 500,
            "busca FTS5 levou {elapsed:?} para {SESSIONS} sessões / {} eventos, acima da meta de 500ms (NFR-008)",
            SESSIONS * EVENTS_PER_SESSION
        );
    }
}
