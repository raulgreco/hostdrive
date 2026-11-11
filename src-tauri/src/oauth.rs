//! OAuth 2.0 com PKCE para Google Drive e OneDrive, usando redirect de
//! loopback (http://127.0.0.1:porta/callback) — o padrão de app desktop.
//!
//! Nenhum segredo circula pelo frontend: o código de autorização chega ao
//! listener local, a troca por tokens acontece aqui em Rust e o resultado
//! vai direto para o chaveiro do sistema operacional.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;
use uuid::Uuid;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";
/// `appDataFolder` = pasta oculta do app no Drive: o HostDrive não polui a
/// pasta "Meu Drive" do usuário e só enxerga o que ele mesmo criou.
const GOOGLE_SCOPES: &str =
    "openid email https://www.googleapis.com/auth/drive.appdata";

const MICROSOFT_AUTH_URL: &str =
    "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
const MICROSOFT_TOKEN_URL: &str =
    "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const MICROSOFT_GRAPH_ME: &str = "https://graph.microsoft.com/v1.0/me";
/// `Files.ReadWrite.AppFolder` = pasta de apps do OneDrive, com a mesma
/// propriedade de isolamento do appDataFolder do Google.
const MICROSOFT_SCOPES: &str = "Files.ReadWrite.AppFolder offline_access User.Read";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthProvider {
    Google,
    OneDrive,
}

impl OAuthProvider {
    pub fn label(self) -> &'static str {
        match self {
            OAuthProvider::Google => "Google Drive",
            OAuthProvider::OneDrive => "OneDrive",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Segundos desde a época; None significa "desconhecido" (trata como
    /// expirado e renova na próxima operação).
    pub expires_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[allow(dead_code)]
    token_type: Option<String>,
    expires_in: Option<i64>,
}

fn pkce_pair() -> (String, String) {
    let verifier = format!("{}{}", Uuid::new_v4(), Uuid::new_v4())
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '.' | '_' | '~'))
        .collect::<String>();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

/// Abre o navegador do usuário no provedor, espera o redirect de loopback
/// com o código e troca o código por tokens. Abre a URL via serviço do SO
/// (`open`/`xdg-open`/`explorer`) para não depender de plugin Tauri extra.
pub async fn authorize(provider: OAuthProvider, client_id: &str) -> Result<TokenSet, String> {
    if client_id.trim().is_empty() {
        return Err(format!(
            "informe o Client ID do {} em Configurações antes de conectar.",
            provider.label()
        ));
    }
    let client_id = client_id.trim();
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("não foi possível abrir a porta local para o login: {error}"))?;
    let redirect_port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{redirect_port}/callback");
    let (verifier, challenge) = pkce_pair();

    let (auth_url, scope) = match provider {
        OAuthProvider::Google => (GOOGLE_AUTH_URL, GOOGLE_SCOPES),
        OAuthProvider::OneDrive => (MICROSOFT_AUTH_URL, MICROSOFT_SCOPES),
    };
    // `access_type=offline&prompt=consent` força o Google a devolver sempre um
    // refresh_token; o Microsoft ignora esses parâmetros (usa `offline_access`
    // no escopo) e os aceita sem efeito, então uma URL só serve aos dois.
    let url = format!(
        "{auth_url}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
        url_encode(client_id),
        url_encode(&redirect_uri),
        url_encode(scope),
        url_encode(&challenge),
    );
    open_in_browser(&url)?;
    let code =
        wait_for_code(listener).map_err(|error| format!("login não concluído: {error}"))?;
    exchange_code(
        provider,
        client_id,
        &redirect_uri,
        &code,
        &verifier,
    )
    .await
}

fn token_set(response: TokenResponse) -> TokenSet {
    let expires_at = response
        .expires_in
        .map(|seconds| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64 + seconds);
    TokenSet {
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        expires_at,
    }
}

pub async fn refresh(provider: OAuthProvider, client_id: &str, refresh_token: &str) -> Result<TokenSet, String> {
    let token_url = match provider {
        OAuthProvider::Google => GOOGLE_TOKEN_URL,
        OAuthProvider::OneDrive => MICROSOFT_TOKEN_URL,
    };
    let body = format!(
        "client_id={}&grant_type=refresh_token&refresh_token={}",
        url_encode(client_id),
        url_encode(refresh_token)
    );
    let client = reqwest::Client::new();
    let response = client
        .post(token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|error| format!("falha ao renovar o token do {}: {error}", provider.label()))?;
    let status = response.status().as_u16();
    let body = response
        .bytes()
        .await
        .map_err(|error| error.to_string())?;
    if !(200..=299).contains(&status) {
        return Err(format!(
            "o {} recusou a renovação do token (HTTP {status}) — conecte novamente.",
            provider.label()
        ));
    }
    let token: TokenResponse =
        serde_json::from_slice(&body).map_err(|error| format!("resposta inesperada do token: {error}"))?;
    Ok(TokenSet {
        access_token: token.access_token,
        // Alguns provedores não devolvem um refresh_token novo na renovação;
        // o chamador mantém o antigo nesse caso.
        refresh_token: token.refresh_token.or_else(|| Some(refresh_token.to_string())),
        expires_at: token
            .expires_in
            .map(|seconds| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64
                    + seconds
            }),
    })
}

/// E-mail da conta conectada — é a "identidade" que aparece na UI e que liga
/// as máquinas do usuário no mesmo espaço de trabalho.
pub async fn account_email(provider: OAuthProvider, access_token: &str) -> Result<String, String> {
    let (url, field) = match provider {
        OAuthProvider::Google => (GOOGLE_USERINFO_URL, "email"),
        OAuthProvider::OneDrive => (MICROSOFT_GRAPH_ME, "userPrincipalName"),
    };
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| format!("falha ao consultar a conta: {error}"))?;
    let status = response.status().as_u16();
    let body = response.bytes().await.map_err(|error| error.to_string())?;
    if !(200..=299).contains(&status) {
        return Err(format!("não foi possível ler a conta (HTTP {status})."));
    }
    let value: serde_json::Value =
        serde_json::from_slice(&body).map_err(|error| format!("resposta inesperada: {error}"))?;
    value[field]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("campo {field} ausente na resposta do provedor."))
}

async fn exchange_code(
    provider: OAuthProvider,
    client_id: &str,
    redirect_uri: &str,
    code: &str,
    verifier: &str,
) -> Result<TokenSet, String> {
    let token_url = match provider {
        OAuthProvider::Google => GOOGLE_TOKEN_URL,
        OAuthProvider::OneDrive => MICROSOFT_TOKEN_URL,
    };
    let body = format!(
        "client_id={}&grant_type=authorization_code&code={}&redirect_uri={}&code_verifier={}",
        url_encode(client_id),
        url_encode(code),
        url_encode(redirect_uri),
        url_encode(verifier)
    );
    let client = reqwest::Client::new();
    let response = client
        .post(token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|error| format!("falha ao trocar o código por token: {error}"))?;
    let status = response.status().as_u16();
    let body = response.bytes().await.map_err(|error| error.to_string())?;
    if !(200..=299).contains(&status) {
        let detail = String::from_utf8_lossy(&body);
        return Err(format!(
            "o {} recusou a troca do código (HTTP {status}): {}",
            provider.label(),
            detail.chars().take(200).collect::<String>()
        ));
    }
    let token: TokenResponse =
        serde_json::from_slice(&body).map_err(|error| format!("resposta inesperada do token: {error}"))?;
    Ok(token_set(token))
}

/// Espera o navegador voltar com `?code=...` na porta local. Responde uma
/// página simples e devolve o código. Timeout: 5 minutos.
fn wait_for_code(listener: TcpListener) -> Result<String, String> {
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(300) {
            return Err("tempo esgotado aguardando o login no navegador.".into());
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut request = [0u8; 8192];
                let length = stream.read(&mut request).unwrap_or(0);
                let text = String::from_utf8_lossy(&request[..length]);
                let code = text
                    .split_whitespace()
                    .find(|part| part.starts_with("GET /callback?"))
                    .and_then(|part| part.split("code=").nth(1))
                    .and_then(|rest| rest.split('&').next())
                    .map(url_decode);
                let page = "<!doctype html><meta charset=utf-8><title>HostDrive</title><body style=\"font-family:system-ui;display:grid;place-items:center;height:100vh;margin:0\"><p style=\"font-size:18px\">Login concluído — você já pode voltar ao HostDrive. ✔</p>";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
                    page.len()
                );
                let _ = stream.write_all(response.as_bytes());
                if let Some(Ok(code)) = code {
                    return Ok(code);
                }
                if let Some(error) = text.split("error=").nth(1).and_then(|rest| rest.split('&').next()) {
                    return Err(format!("o provedor recusou o login ({error})."));
                }
                return Err("resposta inesperada do navegador no redirect.".into());
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(format!("falha ao receber o login: {error}")),
        }
    }
}

fn open_in_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let (program, arg) = ("open", url);
    #[cfg(target_os = "windows")]
    let (program, arg) = ("rundll32", format!("url.dll,FileProtocolHandler {url}"));
    #[cfg(all(unix, not(target_os = "macos")))]
    let (program, arg) = ("xdg-open", url);
    std::process::Command::new(program)
        .arg(&arg)
        .spawn()
        .map_err(|error| format!("não foi possível abrir o navegador: {error}"))?;
    Ok(())
}

/// Codificação de URL mínima e determinística (sem deps extras).
fn url_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            byte => format!("%{byte:02X}"),
        })
        .collect()
}

fn url_decode(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err("percent-encoding incompleto no redirect.".into());
            }
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3])
                .map_err(|_| "percent-encoding inválido no redirect.".to_string())?;
            let byte = u8::from_str_radix(hex, 16)
                .map_err(|_| "percent-encoding inválido no redirect.".to_string())?;
            out.push(byte);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "redirect com texto inválido.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_verifier_is_safe_and_challenge_matches_sha256() {
        let (verifier, challenge) = pkce_pair();
        assert!(verifier.len() >= 43);
        assert!(verifier
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '.' | '_' | '~')));
        let expected = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        assert_eq!(challenge, expected);
    }

    #[test]
    fn url_encode_and_decode_round_trip() {
        let value = "https://exemplo.com/caminho?a=1&b=ção";
        assert_eq!(url_decode(&url_encode(value)).unwrap(), value);
        assert_eq!(url_encode("abc-_.~"), "abc-_.~");
        assert_eq!(url_encode("a b"), "a%20b");
        assert!(url_decode("%2").is_err());
        assert!(url_decode("%zz").is_err());
    }
}
