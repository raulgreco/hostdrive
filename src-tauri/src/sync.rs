//! Sincronização opcional de continuidade via Nextcloud (WebDAV).
//!
//! Ver `docs/decisions/005-nextcloud-sync.md` para o desenho completo. Regras
//! que este módulo nunca quebra (NFR-001/002/003 em `docs/01-REQUISITOS.md`):
//! - Nenhum servidor próprio: fala WebDAV puro com o Nextcloud do usuário.
//! - Nenhuma credencial de agente (Codex/Claude/OpenCode/Kimi) é tocada.
//! - A senha de app do Nextcloud e a frase de criptografia ficam só no
//!   chaveiro do sistema operacional — nunca em texto puro no SQLite nem
//!   devolvidas ao frontend depois de salvas.
//! - O conteúdo sincronizado é cifrado no cliente antes de sair da máquina;
//!   quem administra o servidor Nextcloud nunca vê o conteúdo em claro.
//! - Histórico de terminal só entra se o usuário ligou explicitamente para
//!   aquele projeto (`projects.sync_terminal_history`), nunca por padrão.
//! - Nunca sobrescreve silenciosamente: ver `Decision` e o caminho de
//!   `"conflict"` em `sync_project`.

use crate::projects::{read_app_settings, AppSettings, Project, ProjectRepository};
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::Argon2;
use keyring::Entry;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::State;
use uuid::Uuid;

const KEYRING_SERVICE: &str = "hostdrive-nextcloud";
const KEYRING_APP_PASSWORD_USER: &str = "app-password";
const KEYRING_PASSPHRASE_USER: &str = "encryption-passphrase";
const REMOTE_FOLDER: &str = "HostDrive";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const MIN_PASSPHRASE_LEN: usize = 8;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOutcome {
    /// "disabled" | "first_sync" | "pushed" | "pulled" | "conflict" | "unchanged"
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SyncPayload {
    format: String,
    version: u32,
    updated_at: i64,
    project_name: String,
    memory: Vec<Value>,
    tasks: Vec<Value>,
    settings: SettingsSnapshot,
    handoffs: Vec<Value>,
    terminal_history: Option<Vec<Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SettingsSnapshot {
    theme: String,
    retention_days: i64,
    capture_terminal_events: bool,
}

enum Decision {
    Push,
    Pull(SyncPayload),
}

// ---------------------------------------------------------------- comandos

#[tauri::command]
pub fn has_nextcloud_credentials() -> Result<bool, String> {
    Ok(read_secret(KEYRING_APP_PASSWORD_USER)?.is_some()
        && read_secret(KEYRING_PASSPHRASE_USER)?.is_some())
}

#[tauri::command]
pub fn set_nextcloud_credentials(app_password: String, passphrase: String) -> Result<(), String> {
    if app_password.trim().is_empty() {
        return Err("informe a senha de app (App Password) do Nextcloud".into());
    }
    if passphrase.trim().len() < MIN_PASSPHRASE_LEN {
        return Err(format!(
            "a frase de criptografia deve ter ao menos {MIN_PASSPHRASE_LEN} caracteres"
        ));
    }
    write_secret(KEYRING_APP_PASSWORD_USER, app_password.trim())?;
    write_secret(KEYRING_PASSPHRASE_USER, &passphrase)?;
    Ok(())
}

#[tauri::command]
pub fn clear_nextcloud_credentials() -> Result<(), String> {
    delete_secret(KEYRING_APP_PASSWORD_USER);
    delete_secret(KEYRING_PASSPHRASE_USER);
    Ok(())
}

#[tauri::command]
pub async fn test_nextcloud_connection(repository: State<'_, ProjectRepository>) -> Result<String, String> {
    let settings = read_app_settings(&repository).map_err(db_error)?;
    let config = NextcloudConfig::from_settings(&settings)?;
    let client = reqwest::Client::new();
    ensure_remote_folder(&client, &config).await?;
    Ok(format!(
        "Conexão confirmada com {} — pasta \"{REMOTE_FOLDER}\" pronta.",
        config.base_url
    ))
}

#[tauri::command]
pub async fn sync_project_now(
    repository: State<'_, ProjectRepository>,
    project_id: String,
    prefer: Option<String>,
) -> Result<SyncOutcome, String> {
    sync_project(&repository, &project_id, prefer.as_deref()).await
}

// ------------------------------------------------------------- orquestração

pub(crate) async fn sync_project(
    repository: &ProjectRepository,
    project_id: &str,
    prefer: Option<&str>,
) -> Result<SyncOutcome, String> {
    let settings = read_app_settings(repository).map_err(db_error)?;
    if !settings.nextcloud_sync_enabled {
        return Ok(SyncOutcome {
            status: "disabled".into(),
            message: "Sincronização com Nextcloud está desativada em Configurações.".into(),
        });
    }
    let config = NextcloudConfig::from_settings(&settings)?;
    let passphrase = read_secret(KEYRING_PASSPHRASE_USER)?
        .ok_or_else(|| "frase de criptografia do Nextcloud não configurada".to_string())?;
    let project = repository.project_by_id(project_id).map_err(db_error)?;
    let sync_id = project
        .nextcloud_sync_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let remote_name = format!("{sync_id}.uaisync");

    let local_payload = build_local_payload(repository, &project, &settings)?;
    let client = reqwest::Client::new();
    ensure_remote_folder(&client, &config).await?;
    let remote_bytes = download(&client, &config, &remote_name).await?;
    let now = timestamp();

    let decision = match remote_bytes {
        None => Decision::Push,
        Some(blob) => {
            let remote_payload = decrypt_payload(&passphrase, &blob)?;
            if let Some(choice) = prefer {
                if choice == "remote" {
                    Decision::Pull(remote_payload)
                } else {
                    Decision::Push
                }
            } else if remote_payload.updated_at <= project.last_synced_at {
                // O remoto não mudou desde a última vez que sincronizamos
                // este projeto — seguro enviar o estado local, que é mais
                // recente (ou igual).
                Decision::Push
            } else if local_payload.updated_at <= project.last_synced_at {
                // Só o remoto mudou (em outra máquina) desde então.
                Decision::Pull(remote_payload)
            } else {
                // Os dois lados mudaram desde a última sincronização: nunca
                // decidir isso silenciosamente (NFR-003).
                return Ok(SyncOutcome {
                    status: "conflict".into(),
                    message: format!(
                        "Mudanças em duas máquinas: versão local de {} e versão no Nextcloud de {}. Escolha qual manter.",
                        format_time(local_payload.updated_at),
                        format_time(remote_payload.updated_at)
                    ),
                });
            }
        }
    };

    match decision {
        Decision::Push => {
            let blob = encrypt_payload(&passphrase, &local_payload)?;
            upload(&client, &config, &remote_name, &blob).await?;
            repository
                .mark_project_synced(project_id, &sync_id, now)
                .map_err(db_error)?;
            Ok(SyncOutcome {
                status: "pushed".into(),
                message: "Memória, tarefas, pacotes de transferência e preferências enviados ao Nextcloud.".into(),
            })
        }
        Decision::Pull(payload) => {
            apply_remote_payload(repository, project_id, &payload)?;
            repository
                .mark_project_synced(project_id, &sync_id, now)
                .map_err(db_error)?;
            Ok(SyncOutcome {
                status: "pulled".into(),
                message: "Dados trazidos do Nextcloud (atualizados em outra máquina).".into(),
            })
        }
    }
}

// ------------------------------------------------------------ carga local

fn build_local_payload(
    repository: &ProjectRepository,
    project: &Project,
    settings: &AppSettings,
) -> Result<SyncPayload, String> {
    let connection = repository.open_for_workspace().map_err(db_error)?;

    let mut memory_statement = connection
        .prepare("SELECT section,content,provenance,updated_at FROM project_memory WHERE project_id=?1 ORDER BY section")
        .map_err(db_error)?;
    let memory: Vec<Value> = memory_statement
        .query_map([&project.id], |row| {
            Ok(json!({
                "section": row.get::<_, String>(0)?,
                "content": row.get::<_, String>(1)?,
                "provenance": row.get::<_, String>(2)?,
                "updatedAt": row.get::<_, i64>(3)?,
            }))
        })
        .map_err(db_error)?
        .collect::<Result<_, _>>()
        .map_err(db_error)?;

    let mut task_statement = connection
        .prepare("SELECT id,title,completed,provenance,created_at,updated_at FROM project_tasks WHERE project_id=?1")
        .map_err(db_error)?;
    let tasks: Vec<Value> = task_statement
        .query_map([&project.id], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "completed": row.get::<_, bool>(2)?,
                "provenance": row.get::<_, String>(3)?,
                "createdAt": row.get::<_, i64>(4)?,
                "updatedAt": row.get::<_, i64>(5)?,
            }))
        })
        .map_err(db_error)?
        .collect::<Result<_, _>>()
        .map_err(db_error)?;

    let mut handoff_statement = connection
        .prepare("SELECT id,source_agent,target_agent,status,package_path,created_at FROM handoffs WHERE project_id=?1")
        .map_err(db_error)?;
    let handoff_rows: Vec<(String, String, String, String, String, i64)> = handoff_statement
        .query_map([&project.id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .map_err(db_error)?
        .collect::<Result<_, _>>()
        .map_err(db_error)?;
    let mut handoffs = Vec::with_capacity(handoff_rows.len());
    for (id, source_agent, target_agent, status, package_path, created_at) in handoff_rows {
        let files = read_handoff_files(&package_path);
        handoffs.push(json!({
            "id": id, "sourceAgent": source_agent, "targetAgent": target_agent,
            "status": status, "createdAt": created_at, "files": files,
        }));
    }

    let terminal_history = if project.sync_terminal_history {
        Some(build_terminal_history(&connection, &project.id)?)
    } else {
        None
    };

    let max_memory: i64 = memory.iter().filter_map(|v| v["updatedAt"].as_i64()).max().unwrap_or(0);
    let max_task: i64 = tasks.iter().filter_map(|v| v["updatedAt"].as_i64()).max().unwrap_or(0);
    let max_handoff: i64 = handoffs.iter().filter_map(|v| v["createdAt"].as_i64()).max().unwrap_or(0);
    let max_history: i64 = terminal_history
        .as_ref()
        .map(|sessions| sessions.iter().filter_map(|v| v["startedAt"].as_i64()).max().unwrap_or(0))
        .unwrap_or(0);
    let mut updated_at = max_memory.max(max_task).max(max_handoff).max(max_history);
    if updated_at == 0 {
        updated_at = timestamp();
    }

    Ok(SyncPayload {
        format: "hostdrive-sync".into(),
        version: 1,
        updated_at,
        project_name: project.name.clone(),
        memory,
        tasks,
        settings: SettingsSnapshot {
            theme: settings.theme.clone(),
            retention_days: settings.retention_days,
            capture_terminal_events: settings.capture_terminal_events,
        },
        handoffs,
        terminal_history,
    })
}

fn read_handoff_files(package_path: &str) -> Value {
    const FILES: [&str; 5] = [
        "manifest.json",
        "memory.json",
        "tasks.json",
        "context.md",
        "integrity.json",
    ];
    let mut map = serde_json::Map::new();
    for file in FILES {
        if let Ok(content) = fs::read_to_string(format!("{package_path}/{file}")) {
            map.insert(file.to_string(), Value::String(content));
        }
    }
    Value::Object(map)
}

fn build_terminal_history(
    connection: &rusqlite::Connection,
    project_id: &str,
) -> Result<Vec<Value>, String> {
    let mut session_statement = connection
        .prepare("SELECT id,agent_id,working_directory,status,started_at,ended_at,resumed FROM terminal_sessions WHERE project_id=?1")
        .map_err(db_error)?;
    let sessions: Vec<(String, String, String, String, i64, Option<i64>, bool)> = session_statement
        .query_map([project_id], |row| {
            Ok((
                row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?,
            ))
        })
        .map_err(db_error)?
        .collect::<Result<_, _>>()
        .map_err(db_error)?;

    let mut result = Vec::with_capacity(sessions.len());
    for (id, agent_id, working_directory, status, started_at, ended_at, resumed) in sessions {
        let mut event_statement = connection
            .prepare("SELECT direction,data,created_at FROM terminal_events WHERE session_id=?1 ORDER BY id")
            .map_err(db_error)?;
        let events: Vec<Value> = event_statement
            .query_map([&id], |row| {
                let data: Vec<u8> = row.get(1)?;
                Ok(json!({
                    "direction": row.get::<_, String>(0)?,
                    "text": String::from_utf8_lossy(&data).into_owned(),
                    "createdAt": row.get::<_, i64>(2)?,
                }))
            })
            .map_err(db_error)?
            .collect::<Result<_, _>>()
            .map_err(db_error)?;
        result.push(json!({
            "sessionId": id, "agentId": agent_id, "workingDirectory": working_directory,
            "status": status, "startedAt": started_at, "endedAt": ended_at, "resumed": resumed,
            "events": events,
        }));
    }
    Ok(result)
}

// --------------------------------------------------------- aplicar remoto

fn apply_remote_payload(
    repository: &ProjectRepository,
    project_id: &str,
    payload: &SyncPayload,
) -> Result<(), String> {
    let connection = repository.open_for_workspace().map_err(db_error)?;

    for entry in &payload.memory {
        let (Some(section), Some(content), Some(updated_at)) = (
            entry["section"].as_str(),
            entry["content"].as_str(),
            entry["updatedAt"].as_i64(),
        ) else {
            continue;
        };
        let provenance = entry["provenance"].as_str().unwrap_or("USER_CONFIRMED");
        connection.execute(
            "INSERT INTO project_memory(project_id,section,content,provenance,updated_at) VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(project_id,section) DO UPDATE SET content=excluded.content,provenance=excluded.provenance,updated_at=excluded.updated_at
             WHERE excluded.updated_at >= project_memory.updated_at",
            params![project_id, section, content, provenance, updated_at],
        ).map_err(db_error)?;
    }

    for entry in &payload.tasks {
        let (Some(id), Some(title), Some(completed), Some(created_at), Some(updated_at)) = (
            entry["id"].as_str(),
            entry["title"].as_str(),
            entry["completed"].as_bool(),
            entry["createdAt"].as_i64(),
            entry["updatedAt"].as_i64(),
        ) else {
            continue;
        };
        let provenance = entry["provenance"].as_str().unwrap_or("USER_CONFIRMED");
        connection.execute(
            "INSERT INTO project_tasks(id,project_id,title,completed,provenance,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(id) DO UPDATE SET title=excluded.title,completed=excluded.completed,provenance=excluded.provenance,updated_at=excluded.updated_at
             WHERE excluded.updated_at >= project_tasks.updated_at",
            params![id, project_id, title, completed, provenance, created_at, updated_at],
        ).map_err(db_error)?;
    }

    for entry in &payload.handoffs {
        let (Some(id), Some(source_agent), Some(target_agent), Some(status), Some(created_at)) = (
            entry["id"].as_str(),
            entry["sourceAgent"].as_str(),
            entry["targetAgent"].as_str(),
            entry["status"].as_str(),
            entry["createdAt"].as_i64(),
        ) else {
            continue;
        };
        let exists: bool = connection
            .query_row("SELECT 1 FROM handoffs WHERE id=?1", [id], |_| Ok(true))
            .optional_or_false();
        if exists {
            continue; // pacotes de handoff são imutáveis; não sobrescreve
        }
        let destination = repository.handoff_root().join(id);
        if let Some(files) = entry["files"].as_object() {
            fs::create_dir_all(&destination).map_err(io_error)?;
            for (name, content) in files {
                if let Some(text) = content.as_str() {
                    fs::write(destination.join(name), text).map_err(io_error)?;
                }
            }
        }
        connection.execute(
            "INSERT INTO handoffs(id,project_id,source_agent,target_agent,status,package_path,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![id, project_id, source_agent, target_agent, status, destination.to_string_lossy(), created_at],
        ).map_err(db_error)?;
    }

    if let Some(sessions) = &payload.terminal_history {
        for session in sessions {
            let (Some(session_id), Some(agent_id), Some(working_directory), Some(status), Some(started_at)) = (
                session["sessionId"].as_str(),
                session["agentId"].as_str(),
                session["workingDirectory"].as_str(),
                session["status"].as_str(),
                session["startedAt"].as_i64(),
            ) else {
                continue;
            };
            let exists: bool = connection
                .query_row("SELECT 1 FROM terminal_sessions WHERE id=?1", [session_id], |_| Ok(true))
                .optional_or_false();
            if exists {
                continue; // sessões já são histórico imutável localmente
            }
            let ended_at = session["endedAt"].as_i64();
            let resumed = session["resumed"].as_bool().unwrap_or(false);
            connection.execute(
                "INSERT INTO terminal_sessions(id,project_id,agent_id,working_directory,status,started_at,ended_at,resumed) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                params![session_id, project_id, agent_id, working_directory, status, started_at, ended_at, resumed],
            ).map_err(db_error)?;
            if let Some(events) = session["events"].as_array() {
                for event in events {
                    let (Some(direction), Some(text), Some(created_at)) = (
                        event["direction"].as_str(),
                        event["text"].as_str(),
                        event["createdAt"].as_i64(),
                    ) else {
                        continue;
                    };
                    connection.execute(
                        "INSERT INTO terminal_events(session_id,direction,data,created_at) VALUES(?1,?2,?3,?4)",
                        params![session_id, direction, text.as_bytes(), created_at],
                    ).map_err(db_error)?;
                    let event_id = connection.last_insert_rowid();
                    connection.execute(
                        "INSERT INTO terminal_event_search(rowid,session_id,direction,content) VALUES(?1,?2,?3,?4)",
                        params![event_id, session_id, direction, text],
                    ).map_err(db_error)?;
                }
            }
        }
    }

    connection.execute(
        "UPDATE app_settings SET theme=?1,retention_days=?2,capture_terminal_events=?3 WHERE id=1",
        params![payload.settings.theme, payload.settings.retention_days, payload.settings.capture_terminal_events],
    ).map_err(db_error)?;

    Ok(())
}

trait OptionalOrFalse {
    fn optional_or_false(self) -> bool;
}
impl OptionalOrFalse for Result<bool, rusqlite::Error> {
    fn optional_or_false(self) -> bool {
        matches!(self, Ok(true))
    }
}

// --------------------------------------------------------------- WebDAV

struct NextcloudConfig {
    base_url: String,
    username: String,
    app_password: String,
}

impl NextcloudConfig {
    fn from_settings(settings: &AppSettings) -> Result<Self, String> {
        let server = settings.nextcloud_url.trim().to_string();
        let username = settings.nextcloud_username.trim().to_string();
        if server.is_empty() || username.is_empty() {
            return Err("Nextcloud não configurado: informe o endereço do servidor e o usuário em Configurações.".into());
        }
        let app_password = read_secret(KEYRING_APP_PASSWORD_USER)?
            .ok_or_else(|| "senha de app do Nextcloud não configurada".to_string())?;
        let server = server.trim_end_matches('/');
        let base_url = format!("{server}/remote.php/dav/files/{}", percent_encode(&username));
        Ok(Self { base_url, username, app_password })
    }

    fn folder_url(&self) -> String {
        format!("{}/{}", self.base_url, percent_encode(REMOTE_FOLDER))
    }

    fn object_url(&self, name: &str) -> String {
        format!("{}/{}", self.folder_url(), percent_encode(name))
    }

}

/// Codificação percentual simples (ASCII), suficiente para o nome fixo da
/// pasta, usuários e nomes de arquivo gerados por `Uuid` — nunca recebe
/// texto arbitrário do usuário sem passar por aqui.
fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// `reqwest` (não `ureq`): o MKCOL do WebDAV (RFC 4918) não é um dos
/// métodos "padrão" que `ureq-proto` aceita validar para HTTP/1.1 — ele
/// recusa antes mesmo de tentar a rede. `reqwest`/`hyper` não têm essa
/// lista fechada e enviam qualquer método, como o WebDAV exige.
fn webdav_method(name: &str) -> reqwest::Method {
    reqwest::Method::from_bytes(name.as_bytes()).expect("nome de método WebDAV estático válido")
}

async fn ensure_remote_folder(client: &reqwest::Client, config: &NextcloudConfig) -> Result<(), String> {
    let response = client
        .request(webdav_method("MKCOL"), config.folder_url())
        .basic_auth(&config.username, Some(&config.app_password))
        .send()
        .await
        .map_err(|error| format!("falha ao preparar a pasta \"{REMOTE_FOLDER}\" no Nextcloud: {error}"))?;
    match response.status().as_u16() {
        200..=299 => Ok(()),
        405 => Ok(()), // já existe (Method Not Allowed na coleção)
        401 | 403 => Err("Nextcloud recusou a autenticação (usuário ou senha de app incorretos).".into()),
        status => Err(format!(
            "falha ao preparar a pasta \"{REMOTE_FOLDER}\" no Nextcloud: HTTP {status}"
        )),
    }
}

async fn download(client: &reqwest::Client, config: &NextcloudConfig, name: &str) -> Result<Option<Vec<u8>>, String> {
    let response = client
        .get(config.object_url(name))
        .basic_auth(&config.username, Some(&config.app_password))
        .send()
        .await
        .map_err(|error| format!("falha ao baixar dados do Nextcloud: {error}"))?;
    match response.status().as_u16() {
        200..=299 => {
            let bytes = response
                .bytes()
                .await
                .map_err(|error| format!("falha ao ler resposta do Nextcloud: {error}"))?;
            Ok(Some(bytes.to_vec()))
        }
        404 => Ok(None),
        401 | 403 => Err("Nextcloud recusou a autenticação (usuário ou senha de app incorretos).".into()),
        status => Err(format!("falha ao baixar dados do Nextcloud: HTTP {status}")),
    }
}

async fn upload(client: &reqwest::Client, config: &NextcloudConfig, name: &str, bytes: &[u8]) -> Result<(), String> {
    let response = client
        .put(config.object_url(name))
        .basic_auth(&config.username, Some(&config.app_password))
        .header("Content-Type", "application/octet-stream")
        .body(bytes.to_vec())
        .send()
        .await
        .map_err(|error| format!("falha ao enviar dados ao Nextcloud: {error}"))?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("falha ao enviar dados ao Nextcloud: HTTP {}", response.status().as_u16()))
    }
}

// ------------------------------------------------------------- criptografia

fn encrypt_payload(passphrase: &str, payload: &SyncPayload) -> Result<Vec<u8>, String> {
    let plaintext = serde_json::to_vec(payload).map_err(|error| error.to_string())?;
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let key_bytes = derive_key(passphrase, &salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_slice())
        .map_err(|_| "falha ao cifrar os dados antes do envio".to_string())?;
    let mut blob = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

fn decrypt_payload(passphrase: &str, blob: &[u8]) -> Result<SyncPayload, String> {
    if blob.len() < SALT_LEN + NONCE_LEN {
        return Err("pacote remoto corrompido ou incompleto".into());
    }
    let (salt, rest) = blob.split_at(SALT_LEN);
    let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);
    let key_bytes = derive_key(passphrase, salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
        "não foi possível decifrar os dados do Nextcloud — confira a frase de criptografia".to_string()
    })?;
    serde_json::from_slice(&plaintext).map_err(|error| format!("pacote remoto em formato inesperado: {error}"))
}

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|error| format!("falha ao derivar a chave de criptografia: {error}"))?;
    Ok(key)
}

// ------------------------------------------------------------------ chaveiro

fn read_secret(user: &str) -> Result<Option<String>, String> {
    let entry = Entry::new(KEYRING_SERVICE, user).map_err(|error| error.to_string())?;
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!("falha ao ler o chaveiro do sistema: {error}")),
    }
}

fn write_secret(user: &str, value: &str) -> Result<(), String> {
    let entry = Entry::new(KEYRING_SERVICE, user).map_err(|error| error.to_string())?;
    entry
        .set_password(value)
        .map_err(|error| format!("falha ao gravar no chaveiro do sistema: {error}"))
}

fn delete_secret(user: &str) {
    if let Ok(entry) = Entry::new(KEYRING_SERVICE, user) {
        let _ = entry.delete_credential();
    }
}

// ------------------------------------------------------------------- utils

fn timestamp() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

fn format_time(unix_seconds: i64) -> String {
    format!("{unix_seconds}") // timestamp bruto; a UI formata para exibição local
}

fn db_error(error: rusqlite::Error) -> String {
    format!("falha no banco local: {error}")
}

fn io_error(error: std::io::Error) -> String {
    format!("falha ao gravar pacote de handoff recebido: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload(updated_at: i64) -> SyncPayload {
        SyncPayload {
            format: "hostdrive-sync".into(),
            version: 1,
            updated_at,
            project_name: "Projeto de Teste".into(),
            memory: vec![json!({"section":"CURRENT_STATE","content":"em andamento","provenance":"USER_CONFIRMED","updatedAt":updated_at})],
            tasks: vec![json!({"id":"t1","title":"revisar PR","completed":false,"provenance":"USER_CONFIRMED","createdAt":updated_at,"updatedAt":updated_at})],
            settings: SettingsSnapshot { theme: "dark".into(), retention_days: 90, capture_terminal_events: true },
            handoffs: vec![],
            terminal_history: None,
        }
    }

    #[test]
    fn encrypt_then_decrypt_round_trips_with_the_right_passphrase() {
        let payload = sample_payload(1_700_000_000);
        let blob = encrypt_payload("uma frase longa e secreta", &payload).unwrap();
        // O conteúdo em claro (nome do projeto) nunca deve aparecer nos bytes
        // que efetivamente saem para o Nextcloud — é a garantia de que o
        // servidor nunca vê dados em claro.
        assert!(!blob.windows(payload.project_name.len()).any(|window| window == payload.project_name.as_bytes()));
        let decrypted = decrypt_payload("uma frase longa e secreta", &blob).unwrap();
        assert_eq!(decrypted.project_name, payload.project_name);
        assert_eq!(decrypted.updated_at, payload.updated_at);
        assert_eq!(decrypted.tasks, payload.tasks);
    }

    #[test]
    fn decrypt_fails_loudly_with_the_wrong_passphrase_instead_of_returning_garbage() {
        let payload = sample_payload(1_700_000_000);
        let blob = encrypt_payload("frase-certa-12345", &payload).unwrap();
        let result = decrypt_payload("frase-errada-12345", &blob);
        assert!(result.is_err(), "senha errada nunca deve decifrar com sucesso");
    }

    #[test]
    fn decrypt_rejects_truncated_or_corrupted_blobs() {
        assert!(decrypt_payload("qualquer-frase", &[1, 2, 3]).is_err());
    }

    #[test]
    fn percent_encode_keeps_safe_characters_and_escapes_the_rest() {
        assert_eq!(percent_encode("codex"), "codex");
        assert_eq!(percent_encode("HostDrive"), "HostDrive");
        assert_eq!(percent_encode("pasta com espaço"), "pasta%20com%20espa%C3%A7o");
        assert_eq!(percent_encode("raul.greco-1_2~3"), "raul.greco-1_2~3");
    }

    #[test]
    fn set_nextcloud_credentials_rejects_short_passphrases_and_empty_app_password() {
        assert!(set_nextcloud_credentials("".into(), "frase longa suficiente".into()).is_err());
        assert!(set_nextcloud_credentials("app-pass".into(), "curta".into()).is_err());
    }
}
