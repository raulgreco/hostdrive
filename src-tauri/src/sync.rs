//! Sincronização opcional de continuidade via nuvem do próprio usuário.
//!
//! Desenho original (Nextcloud/WebDAV) em `docs/decisions/005-nextcloud-sync.md`;
//! multi-provedor (Google Drive, OneDrive) e sync de arquivos de projeto em
//! `docs/decisions/007-multi-cloud-sync.md`. Regras que este módulo nunca
//! quebra (NFR-001/002/003 em `docs/01-REQUISITOS.md`):
//! - Nenhum servidor próprio: falamos com o Nextcloud/Google Drive/OneDrive
//!   *da pessoa*, usando a conta que ela já tem. O e-mail da conta é a
//!   identidade que liga as máquinas — sem cadastro HostDrive.
//! - Nenhuma credencial de agente é tocada.
//! - Senhas e tokens ficam só no chaveiro do sistema operacional — nunca em
//!   texto puro no SQLite nem devolvidos ao frontend depois de salvos.
//! - O conteúdo sincronizado é cifrado no cliente antes de sair da máquina
//!   (AES-256-GCM + Argon2); quem administra a nuvem nunca vê o conteúdo.
//! - Histórico de terminal e arquivos de projeto só entram com opt-in
//!   explícito por projeto — nunca por padrão.
//! - Nunca apaga arquivo nem decide conflito silenciosamente.

use crate::oauth::{self, OAuthProvider};
use crate::projects::{read_app_settings, AppSettings, Project, ProjectRepository};
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::Argon2;
use keyring::Entry;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Digest;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::State;
use uuid::Uuid;

const KEYRING_SERVICE: &str = "hostdrive-nextcloud";
const KEYRING_APP_PASSWORD_USER: &str = "app-password";
const KEYRING_PASSPHRASE_USER: &str = "encryption-passphrase";
const KEYRING_GOOGLE_TOKENS: &str = "google-tokens";
const KEYRING_ONEDRIVE_TOKENS: &str = "onedrive-tokens";
const REMOTE_FOLDER: &str = "Eureka42";
const APP_PAYLOAD_NAME: &str = "app.uaisync";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const MIN_PASSPHRASE_LEN: usize = 8;
/// Limite deliberado: é o teto do upload simples do OneDrive (4 MiB) e do
/// Google (5 MiB) — arquivos maiores são pulados com aviso, nunca partidos.
const MAX_SYNC_FILE_BYTES: u64 = 4 * 1024 * 1024;
/// Diretórios que nunca entram no sync de arquivos (build/dependências).
const EXCLUDED_DIRS: [&str; 13] = [
    ".git", "node_modules", "target", "dist", "build", "out", ".next", "__pycache__",
    ".venv", "venv", ".tox", ".mypy_cache", ".pytest_cache",
];
/// Arquivos que podem carregar segredos — fora do sync mesmo com opt-in.
/// O usuário versiona segredos por outros meios (o app não os toca).
fn is_excluded_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with(".env")
        || lower.ends_with(".pem")
        || lower.ends_with(".key")
        || lower.ends_with(".p12")
        || lower.ends_with(".pfx")
        || lower == "credentials"
        || lower.starts_with("id_rsa")
        || lower.starts_with("id_ed25519")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOutcome {
    /// "disabled" | "first_sync" | "pushed" | "pulled" | "conflict" | "unchanged"
    pub status: String,
    pub message: String,
    /// Em conflito de arquivos: caminhos relativos afetados (decisão do
    /// usuário por "manter local" ou "trazer da nuvem").
    #[serde(rename = "conflictFiles")]
    pub conflict_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudStatus {
    pub provider: String,
    pub provider_label: String,
    pub sync_enabled: bool,
    pub configured: bool,
    pub account: String,
    pub has_passphrase: bool,
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

/// Pacote de nível de aplicativo: preferências + agentes personalizados —
/// o que precisa estar igual em todas as máquinas da mesma conta.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AppPayload {
    format: String,
    version: u32,
    updated_at: i64,
    settings: SettingsSnapshot,
    custom_agents: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct FileMeta {
    size: u64,
    mtime: i64,
    sha256: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileManifest {
    format: String,
    version: u32,
    files: BTreeMap<String, FileMeta>,
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
pub fn get_cloud_status(repository: State<'_, ProjectRepository>) -> Result<CloudStatus, String> {
    let settings = read_app_settings(&repository).map_err(db_error)?;
    let provider = settings.cloud_provider.clone();
    let (configured, account) = match provider.as_str() {
        "google" => (stored_tokens(OAuthProvider::Google)?.is_some(), settings.cloud_account.clone()),
        "onedrive" => (stored_tokens(OAuthProvider::OneDrive)?.is_some(), settings.cloud_account.clone()),
        _ => (
            read_secret(KEYRING_APP_PASSWORD_USER)?.is_some(),
            settings.cloud_account.clone(),
        ),
    };
    Ok(CloudStatus {
        provider: provider.clone(),
        provider_label: provider_label(&provider).to_string(),
        sync_enabled: settings.nextcloud_sync_enabled,
        configured,
        account,
        has_passphrase: read_secret(KEYRING_PASSPHRASE_USER)?.is_some(),
    })
}

/// Conecta a conta do provedor escolhido nas configurações: abre o navegador,
/// espera o redirect local, troca o código por tokens, guarda tudo no
/// chaveiro e registra o e-mail da conta como identidade.
#[tauri::command]
pub async fn connect_cloud(repository: State<'_, ProjectRepository>) -> Result<String, String> {
    let settings = read_app_settings(&repository).map_err(db_error)?;
    let (provider, client_id) = match settings.cloud_provider.as_str() {
        "google" => (OAuthProvider::Google, settings.google_client_id.clone()),
        "onedrive" => (OAuthProvider::OneDrive, settings.onedrive_client_id.clone()),
        other => return Err(format!("provedor \"{other}\" não usa OAuth — configure endereço e senha de app do Nextcloud.")),
    };
    let tokens = oauth::authorize(provider, &client_id).await?;
    let account = oauth::account_email(provider, &tokens.access_token).await
        .unwrap_or_else(|_| "conta conectada".to_string());
    store_tokens(provider, &tokens)?;
    repository
        .set_cloud_account(&account)
        .map_err(|error| format!("falha ao registrar a conta: {error}"))?;
    Ok(account)
}

#[tauri::command]
pub fn disconnect_cloud(repository: State<'_, ProjectRepository>) -> Result<(), String> {
    let settings = read_app_settings(&repository).map_err(db_error)?;
    match settings.cloud_provider.as_str() {
        "google" => delete_secret(KEYRING_GOOGLE_TOKENS),
        "onedrive" => delete_secret(KEYRING_ONEDRIVE_TOKENS),
        _ => return Err("para o Nextcloud, remova as credenciais na seção do Nextcloud.".into()),
    }
    repository
        .set_cloud_account("")
        .map_err(|error| format!("falha ao atualizar a conta: {error}"))
}

#[tauri::command]
pub async fn test_cloud_connection(repository: State<'_, ProjectRepository>) -> Result<String, String> {
    let settings = read_app_settings(&repository).map_err(db_error)?;
    let mut remote = build_remote(&settings).await?;
    remote.ensure_root().await?;
    Ok(format!(
        "Conexão confirmada com {} — espaço do HostDrive pronto.",
        provider_label(&settings.cloud_provider)
    ))
}

/// Compatibilidade com o frontend anterior à tela unificada de nuvem.
#[tauri::command]
pub async fn test_nextcloud_connection(repository: State<'_, ProjectRepository>) -> Result<String, String> {
    test_cloud_connection(repository).await
}

#[tauri::command]
pub async fn sync_project_now(
    repository: State<'_, ProjectRepository>,
    project_id: String,
    prefer: Option<String>,
    files_prefer: Option<String>,
) -> Result<SyncOutcome, String> {
    sync_project(
        &repository,
        &project_id,
        prefer.as_deref(),
        files_prefer.as_deref(),
    )
    .await
}

// ------------------------------------------------------------- orquestração

pub(crate) async fn sync_project(
    repository: &ProjectRepository,
    project_id: &str,
    prefer: Option<&str>,
    files_prefer: Option<&str>,
) -> Result<SyncOutcome, String> {
    let settings = read_app_settings(repository).map_err(db_error)?;
    if !settings.nextcloud_sync_enabled {
        return Ok(SyncOutcome {
            status: "disabled".into(),
            message: format!(
                "Sincronização com {} está desativada em Configurações.",
                provider_label(&settings.cloud_provider)
            ),
            conflict_files: vec![],
        });
    }
    let passphrase = read_secret(KEYRING_PASSPHRASE_USER)?
        .ok_or_else(|| "frase de criptografia não configurada — defina-a na Configurações de nuvem.".to_string())?;
    let mut remote = build_remote(&settings).await?;
    remote.ensure_root().await?;
    let project = repository.project_by_id(project_id).map_err(db_error)?;
    let sync_id = project
        .nextcloud_sync_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let remote_name = format!("{sync_id}.uaisync");

    sync_app_payload(repository, &settings, &passphrase, &mut remote).await?;

    let local_payload = build_local_payload(repository, &project, &settings)?;
    let remote_bytes = remote.download(&remote_name).await?;
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
                Decision::Push
            } else if local_payload.updated_at <= project.last_synced_at {
                Decision::Pull(remote_payload)
            } else {
                return Ok(SyncOutcome {
                    status: "conflict".into(),
                    message: format!(
                        "Mudanças em duas máquinas: versão local de {} e versão na nuvem de {}. Escolha qual manter.",
                        format_time(local_payload.updated_at),
                        format_time(remote_payload.updated_at)
                    ),
                    conflict_files: vec![],
                });
            }
        }
    };

    let data_status: &'static str = match decision {
        Decision::Push => {
            let blob = encrypt_payload(&passphrase, &local_payload)?;
            remote.upload(&remote_name, &blob).await?;
            repository
                .mark_project_synced(project_id, &sync_id, now)
                .map_err(db_error)?;
            "pushed"
        }
        Decision::Pull(payload) => {
            apply_remote_payload(repository, project_id, &payload)?;
            repository
                .mark_project_synced(project_id, &sync_id, now)
                .map_err(db_error)?;
            "pulled"
        }
    };

    // Sync de arquivos do projeto (opt-in por projeto, nunca por padrão).
    let files_outcome = if project.files_sync_enabled {
        Some(
            sync_project_files(
                &mut remote,
                repository,
                &project,
                &sync_id,
                &passphrase,
                files_prefer,
            )
            .await?,
        )
    } else {
        None
    };

    Ok(final_outcome(&settings, data_status, files_outcome))
}

fn final_outcome(
    settings: &AppSettings,
    data_status: &'static str,
    files: Option<FilesSyncOutcome>,
) -> SyncOutcome {
    let label = provider_label(&settings.cloud_provider);
    let mut message = match data_status {
        "pushed" => format!("Memória, tarefas, pacotes de transferência e preferências enviados ao {label}."),
        "pulled" => format!("Dados trazidos do {label} (atualizados em outra máquina)."),
        "first_sync" => format!("Primeira sincronização concluída no {label}."),
        _ => "Nada novo para sincronizar.".to_string(),
    };
    let mut conflict_files = Vec::new();
    if let Some(files) = files {
        conflict_files = files.conflicts;
        message.push_str(&format!(
            " Arquivos: {} enviados, {} baixados.",
            files.uploaded, files.downloaded
        ));
        if !files.skipped_large.is_empty() {
            message.push_str(&format!(
                " Pulados por tamanho (>4 MiB): {}.",
                files.skipped_large.join(", ")
            ));
        }
        if !conflict_files.is_empty() {
            return SyncOutcome {
                status: "conflict".into(),
                message: format!(
                    "Arquivos mudaram nas duas pontas — escolha qual versão manter: {}",
                    conflict_files.join(", ")
                ),
                conflict_files,
            };
        }
    }
    SyncOutcome {
        status: data_status.to_string(),
        message,
        conflict_files,
    }
}

#[derive(Debug, Default)]
struct FilesSyncOutcome {
    uploaded: usize,
    downloaded: usize,
    skipped_large: Vec<String>,
    conflicts: Vec<String>,
}

/// Espelho de arquivos do projeto: sobe o que só existe aqui, baixa o que só
/// existe lá, e nunca apaga nada nem resolve divergência sozinho. O estado
/// combinado da última sincronização (base) vive em
/// `<dados do app>/file_manifests/<sync_id>.json` e na nuvem como
/// `<sync_id>.manifest` (cifrado).
async fn sync_project_files(
    remote: &mut Remote,
    repository: &ProjectRepository,
    project: &Project,
    sync_id: &str,
    passphrase: &str,
    prefer: Option<&str>,
) -> Result<FilesSyncOutcome, String> {
    let root = PathBuf::from(&project.path);
    let (local, skipped_large) = walk_project_files(&root)?;
    let manifest_name = format!("{sync_id}.manifest");
    let remote_manifest: Option<FileManifest> = match remote.download(&manifest_name).await? {
        Some(blob) => Some(serde_json::from_slice(&decrypt_bytes(passphrase, &blob)?)
            .map_err(|error| format!("manifesto remoto em formato inesperado: {error}"))?),
        None => None,
    };
    let remote_files = remote_manifest.unwrap_or_default();
    let base_path = repository.manifest_root().join(format!("{sync_id}.files.json"));
    let base: FileManifest = fs::read(&base_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();

    let mut outcome = FilesSyncOutcome {
        skipped_large,
        ..FilesSyncOutcome::default()
    };
    let mut actions: Vec<(String, FileAction)> = Vec::new();
    let mut paths: HashSet<&String> = HashSet::new();
    paths.extend(local.files.keys());
    paths.extend(remote_files.files.keys());

    for path in paths {
        let l = local.files.get(path);
        let r = remote_files.files.get(path);
        let b = base.files.get(path);
        let action = match (l, r, b) {
            (Some(local_meta), None, _) => Some(FileAction::Upload(local_meta.clone())),
            (None, Some(remote_meta), _) => Some(FileAction::Download(remote_meta.clone())),
            (Some(local_meta), Some(remote_meta), None) => {
                if local_meta == remote_meta {
                    None
                } else {
                    Some(FileAction::Conflict)
                }
            }
            (Some(local_meta), Some(remote_meta), Some(base_meta)) => {
                let local_changed = local_meta != base_meta;
                let remote_changed = remote_meta != base_meta;
                match (local_changed, remote_changed) {
                    (false, false) => None,
                    (true, false) if local_meta != remote_meta => {
                        Some(FileAction::Upload(local_meta.clone()))
                    }
                    (false, true) if local_meta != remote_meta => {
                        Some(FileAction::Download(remote_meta.clone()))
                    }
                    (true, true) if local_meta != remote_meta => Some(FileAction::Conflict),
                    _ => None,
                }
            }
            _ => None,
        };
        if let Some(action) = action {
            actions.push((path.clone(), action));
        }
    }

    // Conflitos: aplica a escolha do usuário ou devolve a lista para a UI.
    let mut conflicts = Vec::new();
    for (path, action) in actions {
        match action {
            FileAction::Upload(meta) => {
                upload_project_file(remote, passphrase, sync_id, &root, &path, &meta).await?;
                outcome.uploaded += 1;
            }
            FileAction::Download(meta) => {
                download_project_file(remote, passphrase, sync_id, &root, &path, &meta).await?;
                outcome.downloaded += 1;
            }
            FileAction::Conflict => match prefer {
                Some("remote") => {
                    let meta = remote_files.files.get(&path).cloned().unwrap();
                    download_project_file(remote, passphrase, sync_id, &root, &path, &meta).await?;
                    outcome.downloaded += 1;
                }
                Some(_) => {
                    let meta = local.files.get(&path).cloned().unwrap();
                    upload_project_file(remote, passphrase, sync_id, &root, &path, &meta).await?;
                    outcome.uploaded += 1;
                }
                None => conflicts.push(path),
            },
        }
    }

    if conflicts.is_empty() {
        // Só publica o novo estado combinado quando não há pendência —
        // conflitos mantêm o base antigo para a próxima rodada decidir.
        let merged = merged_manifest(&local, &remote_files);
        let blob = encrypt_bytes(
            passphrase,
            &serde_json::to_vec(&merged).map_err(|error| error.to_string())?,
        )?;
        remote.upload(&manifest_name, &blob).await?;
        if let Some(parent) = base_path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        fs::write(&base_path, serde_json::to_vec(&merged).map_err(|error| error.to_string())?)
            .map_err(io_error)?;
        repository
            .mark_project_files_synced(&project.id, timestamp())
            .map_err(db_error)?;
    }
    outcome.conflicts = conflicts;
    Ok(outcome)
}

enum FileAction {
    Upload(FileMeta),
    Download(FileMeta),
    Conflict,
}

fn merged_manifest(local: &FileManifest, remote: &FileManifest) -> FileManifest {
    // Estado combinado pós-sync: união com preferência pelo mais recente.
    let mut files = local.files.clone();
    for (path, meta) in &remote.files {
        match (files.get(path), local.files.get(path)) {
            (Some(local_meta), Some(l)) if l == meta || l != local_meta => {
                files.insert(path.clone(), meta.clone());
            }
            (None, _) => {
                files.insert(path.clone(), meta.clone());
            }
            _ => {}
        }
    }
    FileManifest {
        format: "hostdrive-files".into(),
        version: 1,
        files,
    }
}

fn remote_file_name(sync_id: &str, path: &str) -> String {
    format!("{sync_id}.files.{}", percent_encode(path))
}

async fn upload_project_file(
    remote: &mut Remote,
    passphrase: &str,
    sync_id: &str,
    root: &Path,
    path: &str,
    _meta: &FileMeta,
) -> Result<(), String> {
    let bytes = fs::read(root.join(path)).map_err(io_error)?;
    let blob = encrypt_bytes(passphrase, &bytes)?;
    remote.upload(&remote_file_name(sync_id, path), &blob).await
}

async fn download_project_file(
    remote: &mut Remote,
    passphrase: &str,
    sync_id: &str,
    root: &Path,
    path: &str,
    _meta: &FileMeta,
) -> Result<(), String> {
    let blob = remote
        .download(&remote_file_name(sync_id, path))
        .await?
        .ok_or_else(|| format!("arquivo \"{path}\" sumiu da nuvem durante o sync"))?;
    let bytes = decrypt_bytes(passphrase, &blob)?;
    let destination = root.join(path);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    fs::write(&destination, bytes).map_err(io_error)
}

fn walk_project_files(root: &Path) -> Result<(FileManifest, Vec<String>), String> {
    let mut manifest = FileManifest {
        format: "hostdrive-files".into(),
        version: 1,
        files: BTreeMap::new(),
    };
    let mut skipped = Vec::new();
    walk_dir(root, root, &mut manifest, &mut skipped)?;
    Ok((manifest, skipped))
}

fn walk_dir(
    root: &Path,
    dir: &Path,
    manifest: &mut FileManifest,
    skipped: &mut Vec<String>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(io_error)?;
    for entry in entries {
        let entry = entry.map_err(io_error)?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(io_error)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if EXCLUDED_DIRS.contains(&name.as_str()) {
                continue;
            }
            walk_dir(root, &path, manifest, skipped)?;
            continue;
        }
        if is_excluded_file(&name) {
            continue;
        }
        let metadata = entry.metadata().map_err(io_error)?;
        let size = metadata.len();
        let relative = path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if size > MAX_SYNC_FILE_BYTES {
            skipped.push(relative);
            continue;
        }
        let bytes = fs::read(&path).map_err(io_error)?;
        let sha256 = format!("{:x}", sha2::Sha256::digest(&bytes));
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or_default();
        manifest.files.insert(relative, FileMeta { size, mtime, sha256 });
    }
    Ok(())
}

// --------------------------------------------------------- carga local

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

// --------------------------------------------------------- nível de app

/// Sincroniza o pacote de nível de aplicativo (preferências + agentes
/// personalizados) com união de agentes por id e preferência para o lado
/// mais recente nas preferências.
async fn sync_app_payload(
    repository: &ProjectRepository,
    settings: &AppSettings,
    passphrase: &str,
    remote: &mut Remote,
) -> Result<(), String> {
    let local = build_app_payload(repository, settings)?;
    let remote_payload: Option<AppPayload> = match remote.download(APP_PAYLOAD_NAME).await? {
        Some(blob) => Some(
            serde_json::from_slice(&decrypt_bytes(passphrase, &blob)?)
                .map_err(|error| format!("pacote de aplicativo remoto em formato inesperado: {error}"))?,
        ),
        None => None,
    };
    let merged = match remote_payload {
        None => local,
        Some(remote) => {
            if remote.updated_at > local.updated_at {
                apply_remote_settings(repository, &remote.settings)?;
            }
            let mut agents = local.custom_agents.clone();
            let known: HashSet<String> = agents
                .iter()
                .filter_map(|agent| agent["id"].as_str().map(str::to_string))
                .collect();
            for agent in &remote.custom_agents {
                if let Some(id) = agent["id"].as_str() {
                    if !known.contains(id) {
                        agents.push(agent.clone());
                    }
                }
            }
            AppPayload {
                updated_at: timestamp(),
                settings: if remote.updated_at > local.updated_at {
                    remote.settings.clone()
                } else {
                    local.settings.clone()
                },
                custom_agents: agents,
                ..local
            }
        }
    };
    // Aplica agentes novos que vieram da nuvem (INSERT OR IGNORE: local ganha
    // em caso de colisão de id — nunca sobrescreve registro local).
    for agent in &merged.custom_agents {
        if let (Some(id), Some(label), Some(executable)) = (
            agent["id"].as_str(),
            agent["label"].as_str(),
            agent["executable"].as_str(),
        ) {
            repository
                .open_for_workspace()
                .map_err(db_error)?
                .execute(
                    "INSERT OR IGNORE INTO custom_agents(id,label,executable,created_at) VALUES(?1,?2,?3,?4)",
                    params![id, label, executable, timestamp()],
                )
                .map_err(db_error)?;
        }
    }
    let blob = encrypt_bytes(
        passphrase,
        &serde_json::to_vec(&merged).map_err(|error| error.to_string())?,
    )?;
    remote.upload(APP_PAYLOAD_NAME, &blob).await?;
    Ok(())
}

fn build_app_payload(
    repository: &ProjectRepository,
    settings: &AppSettings,
) -> Result<AppPayload, String> {
    let connection = repository.open_for_workspace().map_err(db_error)?;
    let mut statement = connection
        .prepare("SELECT id,label,executable FROM custom_agents")
        .map_err(db_error)?;
    let custom_agents: Vec<Value> = statement
        .query_map([], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "label": row.get::<_, String>(1)?,
                "executable": row.get::<_, String>(2)?,
            }))
        })
        .map_err(db_error)?
        .collect::<Result<_, _>>()
        .map_err(db_error)?;
    Ok(AppPayload {
        format: "hostdrive-app".into(),
        version: 1,
        updated_at: timestamp(),
        settings: SettingsSnapshot {
            theme: settings.theme.clone(),
            retention_days: settings.retention_days,
            capture_terminal_events: settings.capture_terminal_events,
        },
        custom_agents,
    })
}

fn apply_remote_settings(repository: &ProjectRepository, settings: &SettingsSnapshot) -> Result<(), String> {
    repository
        .open_for_workspace()
        .map_err(db_error)?
        .execute(
            "UPDATE app_settings SET theme=?1,retention_days=?2,capture_terminal_events=?3 WHERE id=1",
            params![settings.theme, settings.retention_days, settings.capture_terminal_events],
        )
        .map_err(db_error)?;
    Ok(())
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

    apply_remote_settings(repository, &payload.settings)?;
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

// ------------------------------------------------------------------ remoto

enum Remote {
    Nextcloud { config: NextcloudConfig, client: reqwest::Client },
    GoogleDrive { access_token: String, client: reqwest::Client, ids: HashMap<String, String> },
    OneDrive { access_token: String, client: reqwest::Client },
}

async fn build_remote(settings: &AppSettings) -> Result<Remote, String> {
    let client = reqwest::Client::new();
    match settings.cloud_provider.as_str() {
        "google" => {
            let tokens = ensure_tokens(OAuthProvider::Google, &settings.google_client_id).await?;
            Ok(Remote::GoogleDrive { access_token: tokens.access_token, client, ids: HashMap::new() })
        }
        "onedrive" => {
            let tokens = ensure_tokens(OAuthProvider::OneDrive, &settings.onedrive_client_id).await?;
            Ok(Remote::OneDrive { access_token: tokens.access_token, client })
        }
        _ => Ok(Remote::Nextcloud { config: NextcloudConfig::from_settings(settings)?, client }),
    }
}

fn provider_label(provider: &str) -> &'static str {
    match provider {
        "google" => "Google Drive",
        "onedrive" => "OneDrive",
        _ => "Nextcloud",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredTokens {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
}

fn stored_tokens(provider: OAuthProvider) -> Result<Option<StoredTokens>, String> {
    match read_secret(keyring_tokens_user(provider))? {
        Some(json) => serde_json::from_str(&json)
            .map(Some)
            .map_err(|error| format!("tokens armazenados em formato inesperado: {error}")),
        None => Ok(None),
    }
}

fn store_tokens(provider: OAuthProvider, tokens: &oauth::TokenSet) -> Result<(), String> {
    let stored = StoredTokens {
        access_token: tokens.access_token.clone(),
        refresh_token: tokens.refresh_token.clone().unwrap_or_default(),
        expires_at: tokens.expires_at.unwrap_or_else(|| timestamp() + 3600),
    };
    write_secret(
        keyring_tokens_user(provider),
        &serde_json::to_string(&stored).map_err(|error| error.to_string())?,
    )
}

fn keyring_tokens_user(provider: OAuthProvider) -> &'static str {
    match provider {
        OAuthProvider::Google => KEYRING_GOOGLE_TOKENS,
        OAuthProvider::OneDrive => KEYRING_ONEDRIVE_TOKENS,
    }
}

async fn ensure_tokens(provider: OAuthProvider, client_id: &str) -> Result<oauth::TokenSet, String> {
    let stored = stored_tokens(provider)?
        .ok_or_else(|| format!("conta do {} não conectada — conecte em Configurações.", provider.label()))?;
    if stored.expires_at - 60 > timestamp() {
        return Ok(oauth::TokenSet {
            access_token: stored.access_token,
            refresh_token: Some(stored.refresh_token),
            expires_at: Some(stored.expires_at),
        });
    }
    let refreshed = oauth::refresh(provider, client_id, &stored.refresh_token).await?;
    store_tokens(provider, &refreshed)?;
    Ok(refreshed)
}

impl Remote {
    async fn ensure_root(&mut self) -> Result<(), String> {
        match self {
            Remote::Nextcloud { config, client } => {
                let response = client
                    .request(webdav_method("MKCOL"), config.folder_url())
                    .basic_auth(&config.username, Some(&config.app_password))
                    .send()
                    .await
                    .map_err(|error| format!("falha ao preparar a pasta \"{REMOTE_FOLDER}\": {error}"))?;
                match response.status().as_u16() {
                    200..=299 | 405 => Ok(()),
                    401 | 403 => Err("Nextcloud recusou a autenticação (usuário ou senha de app incorretos).".into()),
                    status => Err(format!("falha ao preparar a pasta \"{REMOTE_FOLDER}\" no Nextcloud: HTTP {status}")),
                }
            }
            // Google Drive (appDataFolder) e OneDrive (approot) já existem por
            // definição — nada a criar.
            _ => Ok(()),
        }
    }

    async fn download(&mut self, name: &str) -> Result<Option<Vec<u8>>, String> {
        match self {
            Remote::Nextcloud { config, client } => {
                let response = client
                    .get(config.object_url(name))
                    .basic_auth(&config.username, Some(&config.app_password))
                    .send()
                    .await
                    .map_err(|error| format!("falha ao baixar dados da nuvem: {error}"))?;
                match response.status().as_u16() {
                    200..=299 => {
                        let bytes = response.bytes().await.map_err(|error| format!("falha ao ler resposta: {error}"))?;
                        Ok(Some(bytes.to_vec()))
                    }
                    404 => Ok(None),
                    401 | 403 => Err("Nextcloud recusou a autenticação (usuário ou senha de app incorretos).".into()),
                    status => Err(format!("falha ao baixar dados da nuvem: HTTP {status}")),
                }
            }
            Remote::GoogleDrive { access_token, client, ids } => {
                google_resolve_ids(client, access_token, ids).await?;
                let Some(id) = ids.get(name) else { return Ok(None) };
                let response = client
                    .get(format!("https://www.googleapis.com/drive/v3/files/{id}?alt=media"))
                    .bearer_auth(access_token)
                    .send()
                    .await
                    .map_err(|error| format!("falha ao baixar do Google Drive: {error}"))?;
                match response.status().as_u16() {
                    200..=299 => {
                        let bytes = response.bytes().await.map_err(|error| format!("falha ao ler resposta: {error}"))?;
                        Ok(Some(bytes.to_vec()))
                    }
                    404 => Ok(None),
                    status => Err(format!("falha ao baixar do Google Drive: HTTP {status}")),
                }
            }
            Remote::OneDrive { access_token, client } => {
                let response = client
                    .get(format!(
                        "https://graph.microsoft.com/v1.0/drive/special/approot:/{}:/content",
                        percent_encode(name)
                    ))
                    .bearer_auth(access_token)
                    .send()
                    .await
                    .map_err(|error| format!("falha ao baixar do OneDrive: {error}"))?;
                match response.status().as_u16() {
                    200..=299 => {
                        let bytes = response.bytes().await.map_err(|error| format!("falha ao ler resposta: {error}"))?;
                        Ok(Some(bytes.to_vec()))
                    }
                    404 => Ok(None),
                    status => Err(format!("falha ao baixar do OneDrive: HTTP {status}")),
                }
            }
        }
    }

    async fn upload(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        match self {
            Remote::Nextcloud { config, client } => {
                let response = client
                    .put(config.object_url(name))
                    .basic_auth(&config.username, Some(&config.app_password))
                    .header("Content-Type", "application/octet-stream")
                    .body(bytes.to_vec())
                    .send()
                    .await
                    .map_err(|error| format!("falha ao enviar dados à nuvem: {error}"))?;
                if response.status().is_success() {
                    Ok(())
                } else {
                    Err(format!("falha ao enviar dados à nuvem: HTTP {}", response.status().as_u16()))
                }
            }
            Remote::GoogleDrive { access_token, client, ids } => {
                google_resolve_ids(client, access_token, ids).await?;
                if let Some(id) = ids.get(name) {
                    let response = client
                        .patch(format!("https://www.googleapis.com/upload/drive/v3/files/{id}?uploadType=media"))
                        .bearer_auth(access_token)
                        .header("Content-Type", "application/octet-stream")
                        .body(bytes.to_vec())
                        .send()
                        .await
                        .map_err(|error| format!("falha ao enviar ao Google Drive: {error}"))?;
                    if response.status().is_success() {
                        return Ok(());
                    }
                    return Err(format!("falha ao atualizar arquivo no Google Drive: HTTP {}", response.status().as_u16()));
                }
                // Novo arquivo: multipart (metadata + conteúdo) numa única
                // requisição — limite do upload simples do Google é 5 MiB,
                // compatível com o nosso teto de 4 MiB.
                let boundary = format!("hostdrive{}", Uuid::new_v4());
                let metadata = format!("{{\"name\":\"{}\",\"parents\":[\"appDataFolder\"]}}", name.replace('"', ""));
                let mut body = Vec::with_capacity(bytes.len() + metadata.len() + 256);
                body.extend_from_slice(format!("--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{metadata}\r\n--{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes());
                body.extend_from_slice(bytes);
                body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
                let response = client
                    .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart&fields=id")
                    .bearer_auth(access_token)
                    .header("Content-Type", format!("multipart/related; boundary={boundary}"))
                    .body(body)
                    .send()
                    .await
                    .map_err(|error| format!("falha ao enviar ao Google Drive: {error}"))?;
                let status = response.status().as_u16();
                let body = response.bytes().await.map_err(|error| error.to_string())?;
                if !(200..=299).contains(&status) {
                    return Err(format!("falha ao criar arquivo no Google Drive: HTTP {status}"));
                }
                let created: Value = serde_json::from_slice(&body).map_err(|error| format!("resposta inesperada do Google Drive: {error}"))?;
                if let Some(id) = created["id"].as_str() {
                    ids.insert(name.to_string(), id.to_string());
                }
                Ok(())
            }
            Remote::OneDrive { access_token, client } => {
                let response = client
                    .put(format!(
                        "https://graph.microsoft.com/v1.0/drive/special/approot:/{}:/content",
                        percent_encode(name)
                    ))
                    .bearer_auth(access_token)
                    .header("Content-Type", "application/octet-stream")
                    .body(bytes.to_vec())
                    .send()
                    .await
                    .map_err(|error| format!("falha ao enviar ao OneDrive: {error}"))?;
                if response.status().is_success() {
                    Ok(())
                } else {
                    Err(format!("falha ao enviar ao OneDrive: HTTP {}", response.status().as_u16()))
                }
            }
        }
    }
}

/// Lista os arquivos do appDataFolder e preenche o mapa nome → id (com cache
/// entre chamadas dentro de uma mesma sessão de sync).
async fn google_resolve_ids(
    client: &reqwest::Client,
    access_token: &str,
    ids: &mut HashMap<String, String>,
) -> Result<(), String> {
    if !ids.is_empty() {
        return Ok(());
    }
    let mut next_url: Option<String> = Some(
        "https://www.googleapis.com/drive/v3/files?fields=nextPageToken,files(id,name)&q='appDataFolder'%20in%20parents%20and%20trashed=false".to_string(),
    );
    while let Some(url) = next_url {
        let response = client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| format!("falha ao listar o appDataFolder: {error}"))?;
        let status = response.status().as_u16();
        let body = response.bytes().await.map_err(|error| error.to_string())?;
        if !(200..=299).contains(&status) {
            return Err(format!("falha ao listar o appDataFolder: HTTP {status}"));
        }
        let value: Value = serde_json::from_slice(&body)
            .map_err(|error| format!("resposta inesperada do Google Drive: {error}"))?;
        if let Some(files) = value["files"].as_array() {
            for file in files {
                if let (Some(id), Some(name)) = (file["id"].as_str(), file["name"].as_str()) {
                    ids.insert(name.to_string(), id.to_string());
                }
            }
        }
        next_url = value["nextPageToken"]
            .as_str()
            .map(|token| format!("https://www.googleapis.com/drive/v3/files?pageToken={token}&fields=nextPageToken,files(id,name)&q='appDataFolder'%20in%20parents%20and%20trashed=false"));
    }
    Ok(())
}

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

/// Codificação percentual simples (ASCII), suficiente para nomes gerados
/// pelo app (uuid, slugs, caminhos relativos codificados) — nunca recebe
/// texto arbitrário sem passar por aqui.
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

fn webdav_method(name: &str) -> reqwest::Method {
    reqwest::Method::from_bytes(name.as_bytes()).expect("nome de método WebDAV estático válido")
}

// ------------------------------------------------------------- criptografia

fn encrypt_payload(passphrase: &str, payload: &SyncPayload) -> Result<Vec<u8>, String> {
    let plaintext = serde_json::to_vec(payload).map_err(|error| error.to_string())?;
    encrypt_bytes(passphrase, &plaintext)
}

fn decrypt_payload(passphrase: &str, blob: &[u8]) -> Result<SyncPayload, String> {
    let plaintext = decrypt_bytes(passphrase, blob)?;
    serde_json::from_slice(&plaintext).map_err(|error| format!("pacote remoto em formato inesperado: {error}"))
}

fn encrypt_bytes(passphrase: &str, plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let key_bytes = derive_key(passphrase, &salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| "falha ao cifrar os dados antes do envio".to_string())?;
    let mut blob = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

fn decrypt_bytes(passphrase: &str, blob: &[u8]) -> Result<Vec<u8>, String> {
    if blob.len() < SALT_LEN + NONCE_LEN {
        return Err("pacote remoto corrompido ou incompleto".into());
    }
    let (salt, rest) = blob.split_at(SALT_LEN);
    let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);
    let key_bytes = derive_key(passphrase, salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher.decrypt(nonce, ciphertext).map_err(|_| {
        "não foi possível decifrar os dados da nuvem — confira a frase de criptografia".to_string()
    })
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
        // que efetivamente saem para a nuvem — é a garantia de que o
        // provedor nunca vê dados em claro.
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
        assert!(decrypt_bytes("qualquer-frase", &[1, 2, 3]).is_err());
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

    #[test]
    fn walk_skips_excluded_dirs_and_secret_files_and_huge_files() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        fs::create_dir_all(root.join("src/deep")).unwrap();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::create_dir_all(root.join(".git/objects")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("src/deep/x.txt"), "conteúdo").unwrap();
        fs::write(root.join("node_modules/pkg/index.js"), "nope").unwrap();
        fs::write(root.join(".git/objects/abc"), "nope").unwrap();
        fs::write(root.join(".env"), "SEGREDO=1").unwrap();
        fs::write(root.join("chave.pem"), "-----").unwrap();
        fs::create_dir_all(root.join(".eureka42")).unwrap();
        fs::write(root.join(".eureka42/contexto.md"), "# contexto").unwrap();
        let huge = root.join("grande.bin");
        fs::write(&huge, vec![0u8; (MAX_SYNC_FILE_BYTES + 1) as usize]).unwrap();

        let (manifest, skipped) = walk_project_files(root).unwrap();
        let paths: HashSet<&String> = manifest.files.keys().collect();
        assert!(paths.contains(&"src/main.rs".to_string()));
        assert!(paths.contains(&"src/deep/x.txt".to_string()));
        assert!(paths.contains(&".eureka42/contexto.md".to_string()));
        assert!(!paths.contains(&"node_modules/pkg/index.js".to_string()));
        assert!(!paths.contains(&".git/objects/abc".to_string()));
        assert!(!paths.contains(&".env".to_string()));
        assert!(!paths.contains(&"chave.pem".to_string()));
        assert_eq!(skipped, vec!["grande.bin".to_string()]);
    }

    #[test]
    fn merged_manifest_unions_both_sides() {
        let mut local = FileManifest::default();
        local.files.insert("a.txt".into(), FileMeta { size: 1, mtime: 1, sha256: "a".into() });
        let mut remote = FileManifest::default();
        remote.files.insert("b.txt".into(), FileMeta { size: 2, mtime: 2, sha256: "b".into() });
        let merged = merged_manifest(&local, &remote);
        assert_eq!(merged.files.len(), 2);
        assert!(merged.files.contains_key("a.txt"));
        assert!(merged.files.contains_key("b.txt"));
    }

    #[test]
    fn app_payload_round_trips_through_encryption() {
        let payload = AppPayload {
            format: "hostdrive-app".into(),
            version: 1,
            updated_at: 1_700_000_000,
            settings: SettingsSnapshot { theme: "light".into(), retention_days: 30, capture_terminal_events: false },
            custom_agents: vec![json!({"id":"mycli","label":"Minha CLI","executable":"mycli"})],
        };
        let blob = encrypt_bytes("frase-super-secreta-123", &serde_json::to_vec(&payload).unwrap()).unwrap();
        let decrypted: AppPayload = serde_json::from_slice(&decrypt_bytes("frase-super-secreta-123", &blob).unwrap()).unwrap();
        assert_eq!(decrypted.custom_agents, payload.custom_agents);
        assert_eq!(decrypted.settings.theme, "light");
    }
}
