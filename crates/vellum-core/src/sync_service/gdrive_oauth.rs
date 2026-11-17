//! Authentification OAuth 2.0 PKCE pour l'API Google Drive (sans logiciel de sync local).

use std::fs;
#[cfg(feature = "desktop")]
use std::io::{Read, Write};
#[cfg(feature = "desktop")]
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::Duration;
#[cfg(feature = "desktop")]
use std::time::Instant;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(feature = "desktop")]
use tracing::info;
use tracing::{debug, warn};
use url::Url;
use uuid::Uuid;
use vellum_domain::{VellumError, VellumResult};

const GOOGLE_AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const DEFAULT_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const DRIVE_ABOUT_ENDPOINT: &str =
    "https://www.googleapis.com/drive/v3/about?fields=user(emailAddress)";
const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
#[cfg(feature = "desktop")]
const OAUTH_TIMEOUT_SECS: u64 = 180;

/// Jetons OAuth 2.0 persistés localement pour Google Drive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoogleDriveTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at_unix: i64,
    #[serde(default)]
    pub account_email: Option<String>,
}

impl GoogleDriveTokens {
    pub fn is_access_token_fresh(&self) -> bool {
        self.expires_at_unix > Utc::now().timestamp() + 60
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default = "default_expires_in")]
    expires_in: i64,
}

fn default_expires_in() -> i64 {
    3600
}

#[derive(Debug, Deserialize)]
struct DriveAboutResponse {
    user: Option<DriveAboutUser>,
}

#[derive(Debug, Deserialize)]
struct DriveAboutUser {
    #[serde(rename = "emailAddress")]
    email_address: Option<String>,
}

/// Chemin par défaut du fichier de jetons OAuth Google Drive (`~/.vellum/gdrive_tokens.json`).
pub fn default_tokens_path() -> PathBuf {
    let base = dirs::home_dir()
        .map(|h| h.join(".vellum"))
        .unwrap_or_else(|| PathBuf::from(".vellum"));
    base.join("gdrive_tokens.json")
}

/// Charge les jetons OAuth depuis le disque s'ils existent.
pub fn load_tokens(path: &Path) -> Option<GoogleDriveTokens> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Sauvegarde atomique des jetons OAuth sur disque avec permissions restreintes (`0600` sous Unix).
pub fn save_tokens(path: &Path, tokens: &GoogleDriveTokens) -> VellumResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(tokens)?;
    let temp_path = path.with_extension(format!("tmp_{}", Uuid::new_v4()));
    fs::write(&temp_path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        let _ = fs::set_permissions(&temp_path, perms);
    }

    fs::rename(&temp_path, path)?;
    Ok(())
}

/// Supprime les jetons OAuth stockés localement.
pub fn delete_tokens(path: &Path) -> VellumResult<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Génère une paire PKCE `(code_verifier, code_challenge)` en S256.
pub fn generate_pkce_pair() -> (String, String) {
    let verifier = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(digest);
    (verifier, challenge)
}

/// Construit l'URL d'autorisation OAuth 2.0 Google avec PKCE.
pub fn build_auth_url(
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    code_challenge: &str,
) -> VellumResult<String> {
    let mut url = Url::parse(GOOGLE_AUTH_ENDPOINT)
        .map_err(|e| VellumError::System(format!("URL OAuth invalide: {e}")))?;

    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", DRIVE_SCOPE)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent")
        .append_pair("state", state)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256");

    Ok(url.to_string())
}

/// État d'une session OAuth PKCE mobile (Custom Tabs / ASWebAuthenticationSession).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileOAuthSession {
    pub auth_url: String,
    pub state: String,
    pub code_verifier: String,
    pub redirect_uri: String,
}

/// Prépare une session OAuth 2.0 PKCE pour cible mobile (iOS / Android).
pub fn begin_mobile_oauth(client_id: &str, redirect_uri: &str) -> VellumResult<MobileOAuthSession> {
    let client_id = client_id.trim();
    if client_id.is_empty() {
        return Err(VellumError::InvalidArgument(
            "Le Client ID OAuth Google est requis.".to_string(),
        ));
    }
    let state = Uuid::new_v4().simple().to_string();
    let (code_verifier, code_challenge) = generate_pkce_pair();
    let auth_url = build_auth_url(client_id, redirect_uri, &state, &code_challenge)?;
    Ok(MobileOAuthSession {
        auth_url,
        state,
        code_verifier,
        redirect_uri: redirect_uri.to_string(),
    })
}

/// Finalise une session OAuth mobile après réception du code d'autorisation.
pub fn complete_mobile_oauth(
    client_id: &str,
    client_secret: Option<&str>,
    redirect_uri: &str,
    code: &str,
    code_verifier: &str,
    tokens_path: &Path,
) -> VellumResult<GoogleDriveTokens> {
    let http = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| VellumError::System(format!("Erreur client HTTP OAuth: {e}")))?;

    let mut tokens = exchange_code_for_tokens(
        &http,
        DEFAULT_TOKEN_ENDPOINT,
        client_id.trim(),
        client_secret,
        redirect_uri,
        code,
        code_verifier,
    )?;

    if let Some(email) = fetch_account_email(&http, &tokens.access_token) {
        tokens.account_email = Some(email);
    }

    save_tokens(tokens_path, &tokens)?;
    Ok(tokens)
}

/// Lance le flux OAuth 2.0 PKCE interactif via le navigateur système et un serveur loopback local.
#[cfg(feature = "desktop")]
pub fn authorize_interactive(
    client_id: &str,
    client_secret: Option<&str>,
    tokens_path: &Path,
) -> VellumResult<GoogleDriveTokens> {
    let client_id = client_id.trim();
    if client_id.is_empty() {
        return Err(VellumError::InvalidArgument(
            "Le Client ID OAuth Google est requis.".to_string(),
        ));
    }

    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| {
        VellumError::System(format!(
            "Impossible de démarrer le serveur OAuth local: {e}"
        ))
    })?;
    let port = listener
        .local_addr()
        .map_err(|e| VellumError::System(format!("Port OAuth local introuvable: {e}")))?
        .port();
    listener.set_nonblocking(true).map_err(|e| {
        VellumError::System(format!("Impossible de configurer le socket OAuth: {e}"))
    })?;

    let redirect_uri = format!("http://127.0.0.1:{port}/oauth/callback");
    let state = Uuid::new_v4().simple().to_string();
    let (code_verifier, code_challenge) = generate_pkce_pair();
    let auth_url = build_auth_url(client_id, &redirect_uri, &state, &code_challenge)?;

    info!("Ouverture du navigateur pour l'authentification Google Drive...");
    open::that(&auth_url).map_err(|e| {
        VellumError::System(format!(
            "Impossible d'ouvrir le navigateur pour OAuth Google: {e}"
        ))
    })?;

    let code = wait_for_oauth_callback(&listener, &state, Duration::from_secs(OAUTH_TIMEOUT_SECS))?;

    let http = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| VellumError::System(format!("Erreur client HTTP OAuth: {e}")))?;

    let mut tokens = exchange_code_for_tokens(
        &http,
        DEFAULT_TOKEN_ENDPOINT,
        client_id,
        client_secret,
        &redirect_uri,
        &code,
        &code_verifier,
    )?;

    if let Some(email) = fetch_account_email(&http, &tokens.access_token) {
        tokens.account_email = Some(email);
    }

    save_tokens(tokens_path, &tokens)?;
    Ok(tokens)
}

/// Repli sur cible mobile sans serveur loopback : utiliser `begin_mobile_oauth` + `complete_mobile_oauth`.
#[cfg(not(feature = "desktop"))]
pub fn authorize_interactive(
    _client_id: &str,
    _client_secret: Option<&str>,
    _tokens_path: &Path,
) -> VellumResult<GoogleDriveTokens> {
    Err(VellumError::System(
        "Flux OAuth loopback indisponible sur mobile : utilisez begin_mobile_oauth / complete_mobile_oauth."
            .to_string(),
    ))
}

#[cfg(feature = "desktop")]
fn wait_for_oauth_callback(
    listener: &TcpListener,
    expected_state: &str,
    timeout: Duration,
) -> VellumResult<String> {
    let start = Instant::now();

    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));

                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                let raw_req = String::from_utf8_lossy(&buf[..n]);

                let first_line = raw_req.lines().next().unwrap_or("");
                let target = first_line.split_whitespace().nth(1).unwrap_or("/");

                // Ignorer les requêtes favicon du navigateur
                if target.starts_with("/favicon") {
                    let _ =
                        stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                    continue;
                }

                let full_url = format!("http://127.0.0.1{target}");
                let parsed = Url::parse(&full_url)
                    .map_err(|e| VellumError::System(format!("Réponse OAuth invalide: {e}")))?;

                let mut code: Option<String> = None;
                let mut returned_state: Option<String> = None;
                let mut error: Option<String> = None;

                for (k, v) in parsed.query_pairs() {
                    match k.as_ref() {
                        "code" => code = Some(v.into_owned()),
                        "state" => returned_state = Some(v.into_owned()),
                        "error" => error = Some(v.into_owned()),
                        _ => {}
                    }
                }

                if let Some(err) = error {
                    send_html_response(
                        &mut stream,
                        "400 Bad Request",
                        "Connexion Google Drive refusée",
                        &format!(
                            "Erreur retournée par Google : {err}. Vous pouvez fermer cet onglet."
                        ),
                    );
                    return Err(VellumError::System(format!(
                        "Autorisation Google Drive refusée: {err}"
                    )));
                }

                if returned_state.as_deref() != Some(expected_state) {
                    send_html_response(
                        &mut stream,
                        "400 Bad Request",
                        "État OAuth invalide",
                        "La vérification CSRF (state) a échoué. Veuillez réessayer depuis Vellum.",
                    );
                    return Err(VellumError::System(
                        "État OAuth (state) invalide lors du callback Google.".to_string(),
                    ));
                }

                let Some(auth_code) = code else {
                    send_html_response(
                        &mut stream,
                        "400 Bad Request",
                        "Code manquant",
                        "Aucun code d'autorisation n'a été reçu. Veuillez réessayer.",
                    );
                    return Err(VellumError::System(
                        "Code d'autorisation OAuth manquant.".to_string(),
                    ));
                };

                send_html_response(
                    &mut stream,
                    "200 OK",
                    "Vellum connecté à Google Drive",
                    "Authentification réussie ! Vous pouvez fermer cet onglet et revenir dans Vellum.",
                );
                return Ok(auth_code);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if start.elapsed() >= timeout {
                    return Err(VellumError::System(
                        "Délai d'attente dépassé pour l'authentification Google Drive.".to_string(),
                    ));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(VellumError::System(format!(
                    "Erreur lors de l'attente du callback OAuth: {e}"
                )));
            }
        }
    }
}

#[cfg(feature = "desktop")]
fn send_html_response(stream: &mut std::net::TcpStream, status: &str, title: &str, body: &str) {
    let safe_title = escape_html(title);
    let safe_body = escape_html(body);
    let html = format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{safe_title}</title>\
        <style>body{{font-family:system-ui,sans-serif;background:#0f172a;color:#e2e8f0;display:flex;align-items:center;justify-content:center;height:100vh;margin:0;}}\
        .card{{background:#1e293b;border:1px solid #334155;border-radius:12px;padding:32px;max-width:440px;text-align:center;}}\
        h1{{font-size:20px;margin:0 0 12px;color:#93c5fd;}}p{{font-size:14px;line-height:1.5;color:#cbd5e1;margin:0;}}</style></head>\
        <body><div class=\"card\"><h1>{safe_title}</h1><p>{safe_body}</p></div></body></html>"
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
        html.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

#[cfg(feature = "desktop")]
fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn exchange_code_for_tokens(
    http: &Client,
    token_endpoint: &str,
    client_id: &str,
    client_secret: Option<&str>,
    redirect_uri: &str,
    code: &str,
    code_verifier: &str,
) -> VellumResult<GoogleDriveTokens> {
    let mut form: Vec<(&str, &str)> = vec![
        ("code", code),
        ("client_id", client_id),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
        ("code_verifier", code_verifier),
    ];
    if let Some(secret) = client_secret.map(str::trim).filter(|s| !s.is_empty()) {
        form.push(("client_secret", secret));
    }

    let resp =
        http.post(token_endpoint).form(&form).send().map_err(|e| {
            VellumError::System(format!("Erreur réseau lors de l'échange OAuth: {e}"))
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return Err(VellumError::System(format!(
            "Échec de l'échange de code OAuth ({status}): {text}"
        )));
    }

    let parsed: TokenResponse = resp
        .json()
        .map_err(|e| VellumError::System(format!("Réponse OAuth JSON invalide: {e}")))?;

    Ok(GoogleDriveTokens {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        expires_at_unix: Utc::now().timestamp() + parsed.expires_in,
        account_email: None,
    })
}

fn fetch_account_email(http: &Client, access_token: &str) -> Option<String> {
    let resp = http
        .get(DRIVE_ABOUT_ENDPOINT)
        .bearer_auth(access_token)
        .send()
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let parsed: DriveAboutResponse = resp.json().ok()?;
    parsed.user.and_then(|u| u.email_address)
}

/// Gestionnaire de jetons OAuth avec rafraîchissement automatique.
pub struct TokenManager {
    client_id: String,
    client_secret: Option<String>,
    token_endpoint: String,
    tokens_path: PathBuf,
    tokens: RwLock<Option<GoogleDriveTokens>>,
}

impl TokenManager {
    pub fn new(client_id: String, client_secret: Option<String>, tokens_path: PathBuf) -> Self {
        let loaded = load_tokens(&tokens_path);
        Self {
            client_id,
            client_secret,
            token_endpoint: DEFAULT_TOKEN_ENDPOINT.to_string(),
            tokens_path,
            tokens: RwLock::new(loaded),
        }
    }

    pub fn with_token_endpoint(mut self, endpoint: String) -> Self {
        self.token_endpoint = endpoint;
        self
    }

    pub fn with_initial_tokens(self, tokens: GoogleDriveTokens) -> Self {
        if let Ok(mut guard) = self.tokens.write() {
            *guard = Some(tokens);
        }
        self
    }

    pub fn has_credentials(&self) -> bool {
        if self.client_id.trim().is_empty() {
            return false;
        }
        let Ok(guard) = self.tokens.read() else {
            return false;
        };
        guard
            .as_ref()
            .map(|t| t.refresh_token.is_some() || t.is_access_token_fresh())
            .unwrap_or(false)
    }

    pub fn valid_access_token(&self, http: &Client) -> VellumResult<String> {
        {
            let guard = self.tokens.read().map_err(|e| {
                VellumError::System(format!("Erreur de verrouillage des jetons OAuth: {e}"))
            })?;
            if let Some(ref t) = *guard {
                if t.is_access_token_fresh() {
                    return Ok(t.access_token.clone());
                }
            }
        }

        let refresh_token = {
            let guard = self.tokens.read().map_err(|e| {
                VellumError::System(format!("Erreur de verrouillage des jetons OAuth: {e}"))
            })?;
            guard
                .as_ref()
                .and_then(|t| t.refresh_token.clone())
                .ok_or_else(|| {
                    VellumError::System(
                        "Google Drive n'est pas connecté (aucun refresh_token disponible)."
                            .to_string(),
                    )
                })?
        };

        debug!("Rafraîchissement du jeton d'accès Google Drive...");
        let mut form: Vec<(&str, &str)> = vec![
            ("client_id", self.client_id.trim()),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ];
        if let Some(secret) = self
            .client_secret
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            form.push(("client_secret", secret));
        }

        let resp = http
            .post(&self.token_endpoint)
            .form(&form)
            .send()
            .map_err(|e| {
                VellumError::System(format!(
                    "Erreur réseau lors du rafraîchissement OAuth Google: {e}"
                ))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(VellumError::System(format!(
                "Échec du rafraîchissement OAuth Google ({status}): {text}"
            )));
        }

        let parsed: TokenResponse = resp.json().map_err(|e| {
            VellumError::System(format!("Réponse de rafraîchissement OAuth invalide: {e}"))
        })?;

        let mut updated = {
            let guard = self.tokens.read().map_err(|e| {
                VellumError::System(format!("Erreur de verrouillage des jetons OAuth: {e}"))
            })?;
            guard.clone().unwrap_or(GoogleDriveTokens {
                access_token: String::new(),
                refresh_token: Some(refresh_token.clone()),
                expires_at_unix: 0,
                account_email: None,
            })
        };

        updated.access_token = parsed.access_token.clone();
        if parsed.refresh_token.is_some() {
            updated.refresh_token = parsed.refresh_token;
        }
        updated.expires_at_unix = Utc::now().timestamp() + parsed.expires_in;

        if let Err(e) = save_tokens(&self.tokens_path, &updated) {
            warn!("Impossible de persister le jeton OAuth rafraîchi: {e}");
        }

        if let Ok(mut guard) = self.tokens.write() {
            *guard = Some(updated);
        }

        Ok(parsed.access_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_and_auth_url() {
        let (verifier, challenge) = generate_pkce_pair();
        assert_eq!(verifier.len(), 64);
        assert!(!challenge.is_empty());
        assert!(!challenge.contains('='));

        let url = build_auth_url(
            "my-client-id.apps.googleusercontent.com",
            "http://127.0.0.1:43210/oauth/callback",
            "state123",
            &challenge,
        )
        .unwrap();

        assert!(url.contains("client_id=my-client-id.apps.googleusercontent.com"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("access_type=offline"));
    }

    #[test]
    fn test_tokens_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tokens.json");
        let tokens = GoogleDriveTokens {
            access_token: "acc-123".to_string(),
            refresh_token: Some("ref-456".to_string()),
            expires_at_unix: Utc::now().timestamp() + 3600,
            account_email: Some("reader@example.com".to_string()),
        };

        save_tokens(&path, &tokens).unwrap();
        let loaded = load_tokens(&path).unwrap();
        assert_eq!(loaded, tokens);
        assert!(loaded.is_access_token_fresh());

        delete_tokens(&path).unwrap();
        assert!(load_tokens(&path).is_none());
    }
}
