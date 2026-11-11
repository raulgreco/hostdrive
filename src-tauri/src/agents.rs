use crate::projects::ProjectRepository;
use serde::Serialize;
use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
use tauri::State;
use wait_timeout::ChildExt;

const VERSION_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInstallation {
    pub id: String,
    pub label: String,
    pub executable: String,
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub auth_status: String,
    pub error: Option<String>,
    /// REQ-007: só `true` quando o comando/flag de retomada do próprio
    /// executável já foi validado via `--help` (ver `resume_args`). Nunca
    /// inferir suporte a partir da simples presença do binário.
    pub supports_resume: bool,
    /// `true` quando o agente foi adicionado pelo usuário (tabela
    /// `custom_agents`), em oposição aos do catálogo conhecido.
    pub custom: bool,
}

/// Agente conectado a um projeto — tem ao menos uma sessão de terminal
/// registrada nele. Os não conectados simplesmente não aparecem na UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedAgent {
    pub id: String,
    pub label: String,
    pub executable: String,
    pub session_count: i64,
    pub last_used_at: i64,
    pub running: bool,
}

struct AgentDefinition {
    id: &'static str,
    label: &'static str,
    executable: &'static str,
}

/// Catálogo de CLIs conhecidas. Todos entram na descoberta e podem ser
/// iniciados; `supports_resume` permanece `false` exceto para os quatro cujos
/// comandos de retomada foram validados via `--help` do binário instalado
/// (ver `resume_args` e docs/decisions/003). Usuários podem ainda registrar
/// qualquer outro executável como agente personalizado (`custom_agents`).
const CATALOG: &[AgentDefinition] = &[
    AgentDefinition { id: "codex", label: "Codex", executable: "codex" },
    AgentDefinition { id: "claude", label: "Claude", executable: "claude" },
    AgentDefinition { id: "opencode", label: "OpenCode", executable: "opencode" },
    AgentDefinition { id: "kimi", label: "Kimi", executable: "kimi" },
    AgentDefinition { id: "gemini", label: "Gemini", executable: "gemini" },
    AgentDefinition { id: "aider", label: "Aider", executable: "aider" },
    AgentDefinition { id: "cursor-agent", label: "Cursor", executable: "cursor-agent" },
    AgentDefinition { id: "amp", label: "Amp", executable: "amp" },
    AgentDefinition { id: "goose", label: "Goose", executable: "goose" },
    AgentDefinition { id: "qwen", label: "Qwen Code", executable: "qwen" },
];

/// REQ-007 — argumentos que retomam a sessão interativa nativa mais recente
/// *para o diretório de trabalho atual*. Cada entrada foi validada lendo a
/// ajuda (`--help`) do executável instalado (ver `docs/feasibility.md` e
/// `docs/decisions/003-resume-palette-theme.md`); nenhuma é alegada por
/// suposição — agentes fora desta lista retornam `None` e a UI não oferece
/// retomada para eles. Se a CLI mudar essa interface em versão futura, o
/// comando falhará de forma visível, não silenciosamente.
pub(crate) fn resume_args(agent_id: &str) -> Option<&'static [&'static str]> {
    match agent_id {
        "codex" => Some(&["resume", "--last"]),
        "claude" => Some(&["--continue"]),
        "opencode" => Some(&["--continue"]),
        "kimi" => Some(&["--continue"]),
        _ => None,
    }
}

/// Descobre os agentes instalados nesta máquina: catálogo conhecido +
/// agentes personalizados registrados pelo usuário. Apenas executáveis
/// encontrados no PATH/diretórios conhecidos são retornados — os ausentes
/// não geram item fantasma na UI.
pub(crate) fn discover_agents(repository: &ProjectRepository) -> Vec<AgentInstallation> {
    let customs = repository.list_custom_agents().unwrap_or_default();
    let mut discovered: Vec<AgentInstallation> = CATALOG
        .iter()
        .map(|definition| detect(definition.id, definition.label, definition.executable, false))
        .filter(|installation| installation.available)
        .collect();
    for custom in customs {
        let installation = detect(&custom.id, &custom.label, &custom.executable, true);
        if installation.available {
            discovered.push(installation);
        }
    }
    discovered
}

#[tauri::command]
pub fn detect_agents(repository: State<'_, ProjectRepository>) -> Vec<AgentInstallation> {
    discover_agents(&repository)
}

/// Agentes conectados a um projeto: qualquer `agent_id` com ao menos uma
/// sessão registrada. É a lista que a UI mostra — "as que não conectei não
/// aparecem". Nomes vêm do catálogo ou do registro personalizado; agentes
/// desconhecidos (ex.: histórico vindo de outra máquina) usam o próprio id.
#[tauri::command]
pub fn list_project_agents(
    repository: State<'_, ProjectRepository>,
    project_id: String,
) -> Result<Vec<ConnectedAgent>, String> {
    let rows = repository
        .list_connected_agents(&project_id)
        .map_err(|error| format!("falha ao listar agentes do projeto: {error}"))?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let (label, executable) = describe_agent(&repository, &row.agent_id);
            ConnectedAgent {
                id: row.agent_id,
                label,
                executable,
                session_count: row.session_count,
                last_used_at: row.last_used_at,
                running: row.running,
            }
        })
        .collect())
}

/// Registra um agente personalizado: qualquer executável resolvível no PATH
/// (com as regras de `PATHEXT` no Windows). O id é um slug único derivado do
/// nome do executável, estável e digitável no `///agente <id>`.
#[tauri::command]
pub fn add_custom_agent(
    repository: State<'_, ProjectRepository>,
    label: String,
    executable: String,
) -> Result<AgentInstallation, String> {
    let label = label.trim();
    let executable = executable.trim();
    if label.is_empty() || executable.is_empty() {
        return Err("informe um nome e um comando para o agente.".into());
    }
    if find_executable(executable).is_none() {
        return Err(format!(
            "não encontrei o executável \"{executable}\" no PATH desta máquina."
        ));
    }
    let id = unique_custom_id(&repository, executable);
    repository
        .insert_custom_agent(&id, label, executable)
        .map_err(|error| format!("falha ao registrar agente: {error}"))?;
    let installation = detect(&id, label, executable, true);
    if !installation.available {
        return Err(format!("falha ao detectar o agente \"{id}\" após o registro."));
    }
    Ok(installation)
}

#[tauri::command]
pub fn remove_custom_agent(
    repository: State<'_, ProjectRepository>,
    id: String,
) -> Result<(), String> {
    if CATALOG.iter().any(|definition| definition.id == id) {
        return Err("agentes do catálogo não podem ser removidos.".into());
    }
    repository
        .delete_custom_agent(&id)
        .map_err(|error| format!("falha ao remover agente: {error}"))
}

fn detect(id: &str, label: &str, executable: &str, custom: bool) -> AgentInstallation {
    let supports_resume = resume_args(id).is_some();
    let Some(path) = find_executable(executable) else {
        return AgentInstallation {
            id: id.to_string(),
            label: label.to_string(),
            executable: executable.to_string(),
            available: false,
            path: None,
            version: None,
            auth_status: "UNKNOWN".to_string(),
            error: None,
            supports_resume,
            custom,
        };
    };
    let (version, error) = read_version(&path);
    AgentInstallation {
        id: id.to_string(),
        label: label.to_string(),
        executable: executable.to_string(),
        available: true,
        path: Some(path.to_string_lossy().into_owned()),
        version,
        auth_status: "UNKNOWN".to_string(),
        error,
        supports_resume,
        custom,
    }
}

fn describe_agent(repository: &ProjectRepository, id: &str) -> (String, String) {
    if let Some(definition) = CATALOG.iter().find(|definition| definition.id == id) {
        return (definition.label.to_string(), definition.executable.to_string());
    }
    if let Ok(customs) = repository.list_custom_agents() {
        if let Some(custom) = customs.into_iter().find(|custom| custom.id == id) {
            return (custom.label, custom.executable);
        }
    }
    (id.to_string(), id.to_string())
}

pub(crate) fn find_agent_executable(
    repository: &ProjectRepository,
    agent_id: &str,
) -> Option<PathBuf> {
    let executable = CATALOG
        .iter()
        .find(|definition| definition.id == agent_id)
        .map(|definition| definition.executable)?
        .to_string();
    find_executable(&executable).or_else(|| {
        repository
            .list_custom_agents()
            .ok()?
            .into_iter()
            .find(|custom| custom.id == agent_id)
            .and_then(|custom| find_executable(&custom.executable))
    })
}

/// Id estável e digitável para agente personalizado: slug do executável,
/// sem colidir com catálogo ou outros personalizados (mycli-2, mycli-3…).
fn unique_custom_id(repository: &ProjectRepository, executable: &str) -> String {
    let base = slugify(executable).unwrap_or_else(|| "agente".to_string());
    let existing: HashSet<String> = repository
        .list_custom_agents()
        .map(|customs| customs.into_iter().map(|custom| custom.id).collect())
        .unwrap_or_default();
    let catalog_ids: HashSet<&str> = CATALOG.iter().map(|definition| definition.id).collect();
    if !existing.contains(&base) && !catalog_ids.contains(base.as_str()) {
        return base;
    }
    for suffix in 2.. {
        let candidate = format!("{base}-{suffix}");
        if !existing.contains(&candidate) && !catalog_ids.contains(candidate.as_str()) {
            return candidate;
        }
    }
    unreachable!("suffixes are unbounded")
}

fn slugify(input: &str) -> Option<String> {
    let mut out = String::with_capacity(input.len());
    let mut pending_dash = false;
    for character in input.chars() {
        if character.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(character.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }
    (!out.is_empty()).then_some(out)
}

fn find_executable(name: &str) -> Option<PathBuf> {
    let candidate_names = candidate_file_names(name);
    executable_search_dirs().into_iter().find_map(|dir| {
        candidate_names.iter().find_map(|candidate_name| {
            let candidate = dir.join(candidate_name);
            if candidate.is_file() {
                candidate.canonicalize().ok().or(Some(candidate))
            } else {
                None
            }
        })
    })
}

/// No Windows, `Command`/`CreateProcess` não resolve extensão sozinho como o
/// `cmd.exe` faz: procurar só por "codex" nunca bate com o shim real
/// instalado (`codex.exe`, `claude.cmd`, `opencode.cmd`, `kimi.cmd`, ...),
/// então nenhum agente era encontrado nem podia ser iniciado — era a causa
/// raiz do "não funciona no Windows" (nenhum login é possível se o processo
/// nunca chega a abrir). Em Unix o nome puro continua sendo a única opção.
fn candidate_file_names(name: &str) -> Vec<String> {
    if !cfg!(windows) {
        return vec![name.to_string()];
    }
    let pathext = env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
    let mut names = vec![name.to_string()];
    for extension in pathext.split(';') {
        let extension = extension.trim();
        if extension.is_empty() {
            continue;
        }
        names.push(format!("{name}{}", extension.to_lowercase()));
    }
    names
}

fn executable_search_dirs() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    if let Some(path) = env::var_os("PATH") {
        paths.extend(env::split_paths(&path));
    }
    if cfg!(windows) {
        if let Some(user_profile) = env::var_os("USERPROFILE") {
            let user_profile = PathBuf::from(user_profile);
            paths.extend([
                user_profile.join(".cargo\\bin"),
                user_profile.join(".local\\bin"),
                user_profile.join(".kimi-code\\bin"),
            ]);
        }
        if let Some(appdata) = env::var_os("APPDATA") {
            paths.push(PathBuf::from(appdata).join("npm"));
        }
        if let Some(local_appdata) = env::var_os("LOCALAPPDATA") {
            paths.push(PathBuf::from(local_appdata).join("Programs"));
        }
    } else {
        paths.extend([
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
        ]);
        if let Some(home) = env::var_os("HOME") {
            let home = PathBuf::from(home);
            paths.extend([
                home.join(".local/bin"),
                home.join(".kimi-code/bin"),
                home.join(".cargo/bin"),
            ]);
        }
    }
    paths
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

fn read_version(path: &Path) -> (Option<String>, Option<String>) {
    let child = Command::new(path)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let Ok(mut child) = child else {
        return (None, Some("não foi possível executar --version".into()));
    };

    match child.wait_timeout(VERSION_TIMEOUT) {
        Ok(Some(_)) => match child.wait_with_output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let version =
                    first_non_empty_line(&stdout).or_else(|| first_non_empty_line(&stderr));
                (version.map(str::to_owned), None)
            }
            Err(_) => (None, Some("falha ao coletar a versão".into())),
        },
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            (None, Some("timeout ao consultar a versão".into()))
        }
        Err(_) => (None, Some("falha ao aguardar o processo de versão".into())),
    }
}

fn first_non_empty_line(value: &str) -> Option<&str> {
    value.lines().map(str::trim).find(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn unix_only_tries_the_bare_executable_name() {
        assert_eq!(candidate_file_names("codex"), vec!["codex".to_string()]);
    }

    #[cfg(windows)]
    #[test]
    fn windows_tries_pathext_variants_so_npm_and_cargo_shims_resolve() {
        let names = candidate_file_names("codex");
        assert!(names.contains(&"codex".to_string()));
        assert!(names.contains(&"codex.exe".to_string()));
        assert!(names.contains(&"codex.cmd".to_string()));
    }

    #[test]
    fn selects_first_non_empty_version_line() {
        assert_eq!(
            first_non_empty_line("\n  codex-cli 0.154.0\nnext"),
            Some("codex-cli 0.154.0")
        );
    }

    #[test]
    fn catalog_has_unique_ids_and_executables() {
        let ids: HashSet<_> = CATALOG.iter().map(|agent| agent.id).collect();
        let executables: HashSet<_> = CATALOG.iter().map(|agent| agent.executable).collect();
        assert_eq!(ids.len(), CATALOG.len());
        assert_eq!(executables.len(), CATALOG.len());
    }

    #[test]
    fn resume_is_only_claimed_for_verified_agents_and_unknown_ids_are_rejected() {
        for id in ["codex", "claude", "opencode", "kimi"] {
            assert!(resume_args(id).is_some(), "{id} deveria ter retomada verificada");
        }
        for agent in CATALOG.iter() {
            if ["codex", "claude", "opencode", "kimi"].contains(&agent.id) {
                continue;
            }
            assert!(
                resume_args(agent.id).is_none(),
                "{} não tem retomada verificada e não deve alegar suporte",
                agent.id
            );
        }
        assert!(resume_args("not-a-real-agent").is_none());
    }

    #[test]
    fn slugify_produces_stable_lowercase_ids() {
        assert_eq!(slugify("MyCLI"), Some("mycli".to_string()));
        assert_eq!(slugify("my_tool.v2"), Some("my-tool-v2".to_string()));
        assert_eq!(slugify("  "), None);
        assert_eq!(slugify("curl"), Some("curl".to_string()));
    }

    #[test]
    fn custom_agent_ids_never_collide_with_catalog_or_each_other() {
        let directory = tempfile::tempdir().unwrap();
        let repository = ProjectRepository::new(directory.path().join("test.sqlite"));
        repository.initialize().unwrap();
        let first = unique_custom_id(&repository, "codex");
        assert_eq!(first, "codex-2", "codex é do catálogo; o personalizado precisa de sufixo");
        repository.insert_custom_agent(&first, "Codex local", "codex").unwrap();
        let second = unique_custom_id(&repository, "codex");
        assert_eq!(second, "codex-3");
        let plain = unique_custom_id(&repository, "mycli");
        assert_eq!(plain, "mycli");
    }
}
