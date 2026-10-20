use serde::Serialize;
use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
use wait_timeout::ChildExt;

const VERSION_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInstallation {
    pub id: &'static str,
    pub label: &'static str,
    pub executable: &'static str,
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub auth_status: &'static str,
    pub error: Option<String>,
    /// REQ-007: só `true` quando o comando/flag de retomada do próprio
    /// executável já foi validado via `--help` (ver `resume_args`). Nunca
    /// inferir suporte a partir da simples presença do binário.
    pub supports_resume: bool,
}

struct AgentDefinition {
    id: &'static str,
    label: &'static str,
    executable: &'static str,
}

const AGENTS: [AgentDefinition; 4] = [
    AgentDefinition {
        id: "codex",
        label: "Codex",
        executable: "codex",
    },
    AgentDefinition {
        id: "claude",
        label: "Claude",
        executable: "claude",
    },
    AgentDefinition {
        id: "opencode",
        label: "OpenCode",
        executable: "opencode",
    },
    AgentDefinition {
        id: "kimi",
        label: "Kimi",
        executable: "kimi",
    },
];

/// REQ-007 — argumentos que retomam a sessão interativa nativa mais recente
/// *para o diretório de trabalho atual*, sem exigir que o app rastreie um
/// identificador de sessão próprio. Cada entrada foi validada lendo a ajuda
/// (`--help`) do executável instalado em 2026-10-13 (ver `docs/feasibility.md`
/// e `docs/decisions/003-resume-palette-theme.md`); nenhuma é alegada por
/// suposição. Se a CLI mudar essa interface em versão futura, o comando
/// falhará de forma visível (o processo retornará erro), não silenciosamente.
pub(crate) fn resume_args(agent_id: &str) -> Option<&'static [&'static str]> {
    match agent_id {
        // `codex resume --last`: continua a sessão mais recente; o filtro por
        // cwd já é o padrão (sem `--all`).
        "codex" => Some(&["resume", "--last"]),
        // `claude --continue` (`-c`): "Continue the most recent conversation
        // in the current directory".
        "claude" => Some(&["--continue"]),
        // `opencode --continue` (`-c`) no comando padrão (TUI): "continue the
        // last session".
        "opencode" => Some(&["--continue"]),
        // `kimi --continue` (`-c`): "Continue the previous session for the
        // working directory".
        "kimi" => Some(&["--continue"]),
        _ => None,
    }
}

#[tauri::command]
pub fn detect_agents() -> Vec<AgentInstallation> {
    AGENTS.iter().map(detect_agent).collect()
}

fn detect_agent(definition: &AgentDefinition) -> AgentInstallation {
    let supports_resume = resume_args(definition.id).is_some();
    let Some(path) = find_executable(definition.executable) else {
        return AgentInstallation {
            id: definition.id,
            label: definition.label,
            executable: definition.executable,
            available: false,
            path: None,
            version: None,
            auth_status: "UNKNOWN",
            error: None,
            supports_resume,
        };
    };

    let (version, error) = read_version(&path);
    AgentInstallation {
        id: definition.id,
        label: definition.label,
        executable: definition.executable,
        available: true,
        path: Some(path.to_string_lossy().into_owned()),
        version,
        auth_status: "UNKNOWN",
        error,
        supports_resume,
    }
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

pub(crate) fn find_agent_executable(agent_id: &str) -> Option<PathBuf> {
    AGENTS
        .iter()
        .find(|agent| agent.id == agent_id)
        .and_then(|agent| find_executable(agent.executable))
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
            // Local de instalação padrão dos shims do `npm install -g` no
            // Windows (geralmente já está no PATH, mas é defensivo repetir).
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
    fn registry_has_unique_ids_and_executables() {
        let ids: HashSet<_> = AGENTS.iter().map(|agent| agent.id).collect();
        let executables: HashSet<_> = AGENTS.iter().map(|agent| agent.executable).collect();
        assert_eq!(ids.len(), AGENTS.len());
        assert_eq!(executables.len(), AGENTS.len());
    }

    #[test]
    fn resume_is_only_claimed_for_verified_agents_and_unknown_ids_are_rejected() {
        for agent in AGENTS.iter() {
            assert!(
                resume_args(agent.id).is_some(),
                "{} deveria ter retomada verificada",
                agent.id
            );
        }
        assert!(resume_args("not-a-real-agent").is_none());
    }

    #[test]
    fn detection_returns_a_safe_record_for_every_supported_agent() {
        let detected = detect_agents();
        assert_eq!(detected.len(), AGENTS.len());
        assert!(detected.iter().all(|agent| agent.auth_status == "UNKNOWN"));

        for agent in detected {
            println!(
                "{} available={} path={} version={}",
                agent.id,
                agent.available,
                agent.path.as_deref().unwrap_or("—"),
                agent.version.as_deref().unwrap_or("—")
            );
        }
    }
}
