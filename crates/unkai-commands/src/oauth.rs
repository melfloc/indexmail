//! OAuth2 / XOAUTH2 support for Gmail and Outlook.com.
//!
//! Gmail and Microsoft have both moved IMAP/SMTP away from account
//! passwords. Gmail still offers App Passwords for some accounts, but
//! Outlook.com/Hotmail now requires Modern Auth for every personal
//! account. This module implements the desktop OAuth flow (loopback
//! redirect + PKCE) and stores the long-lived refresh token in the OS
//! keychain. The short-lived access token is kept in memory and handed
//! to `unkai-imap` / `unkai-smtp` through the SASL XOAUTH2 mechanism.
//!
//! Two providers share the flow:
//!   - Google: scope `https://mail.google.com/`, auth/token endpoints on
//!     `accounts.google.com` / `oauth2.googleapis.com`.
//!   - Microsoft: IMAP + SMTP scopes on `outlook.office.com`, endpoints
//!     on `login.microsoftonline.com/common`.
//!
//! Security notes:
//!   - The local callback listener binds loopback only.
//!   - `state` and PKCE S256 are mandatory.
//!   - Tokens and client secrets never enter logs or `accounts.json`;
//!     the refresh token + client config live in the keychain.
//!   - Google's Desktop client has a non-confidential client secret;
//!     Microsoft public clients do not need one.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use axum::{
    Router,
    extract::{Query, State},
    response::Html,
    routing::get,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, oneshot};
use tokio::task::JoinHandle;
use unkai_core::UnkaiError;
use unkai_imap::ImapClient;
use unkai_smtp::SmtpClient;
use unkai_store::credentials;
use uuid::Uuid;

const CALLBACK_PATH: &str = "/oauth2/callback";
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);
const TOKEN_TIMEOUT: Duration = Duration::from_secs(30);
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_SCOPE: &str = "https://mail.google.com/";
const MICROSOFT_AUTH_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
const MICROSOFT_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const MICROSOFT_IMAP_SCOPE: &str = "https://outlook.office.com/IMAP.AccessAsUser.All";
const MICROSOFT_SMTP_SCOPE: &str = "https://outlook.office.com/SMTP.Send";

/// Mail providers with a supported OAuth2/XOAUTH2 flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OAuthProvider {
    Google,
    Microsoft,
}

impl OAuthProvider {
    pub fn id(self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Microsoft => "microsoft",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Google => "Google",
            Self::Microsoft => "Microsoft",
        }
    }

    fn auth_url(self) -> &'static str {
        match self {
            Self::Google => GOOGLE_AUTH_URL,
            Self::Microsoft => MICROSOFT_AUTH_URL,
        }
    }

    fn token_url(self) -> &'static str {
        match self {
            Self::Google => GOOGLE_TOKEN_URL,
            Self::Microsoft => MICROSOFT_TOKEN_URL,
        }
    }

    fn redirect_host(self) -> &'static str {
        // Google accepts either loopback literal. Microsoft's desktop
        // registration documents `http://localhost` (any port) and is
        // stricter about `127.0.0.1`, so use localhost there and serve
        // both address families.
        match self {
            Self::Google => "127.0.0.1",
            Self::Microsoft => "localhost",
        }
    }

    /// Microsoft matches the registered `http://localhost` redirect
    /// exactly except for the port; the path has to be root. Google's
    /// desktop client accepts an arbitrary loopback path.
    fn callback_path(self) -> &'static str {
        // Both providers accept a loopback redirect at the root path.
        // Microsoft registers `http://localhost` (any port) and Google's
        // desktop client accepts the loopback literal with `/`; keeping
        // the path empty avoids provider-specific path matching rules.
        "/"
    }

    fn scopes(self) -> &'static [&'static str] {
        match self {
            Self::Google => &[GOOGLE_SCOPE],
            Self::Microsoft => &[
                MICROSOFT_IMAP_SCOPE,
                MICROSOFT_SMTP_SCOPE,
                "offline_access",
                "openid",
                "profile",
                "email",
            ],
        }
    }

    /// Scope sent on the authorization-code exchange and refresh calls.
    ///
    /// Microsoft's token endpoint requires a *single* resource in the
    /// scope when redeeming a code that spanned several resources. IMAP
    /// and SMTP both live under `outlook.office.com`, so we redeem with
    /// those two scopes only. `offline_access` was already part of the
    /// authorization request; the refresh token is issued from that.
    fn token_scope(self) -> Option<&'static str> {
        match self {
            Self::Google => None,
            Self::Microsoft => Some(
                "https://outlook.office.com/IMAP.AccessAsUser.All \
                 https://outlook.office.com/SMTP.Send",
            ),
        }
    }
}

/// JSON blob persisted in the keychain for an OAuth2 account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCredentials {
    pub provider: OAuthProvider,
    /// Public desktop-client id from the provider's developer console.
    pub client_id: String,
    /// Google desktop clients have a non-confidential secret; Microsoft
    /// public clients leave this empty.
    #[serde(default)]
    pub client_secret: String,
    pub refresh_token: String,
    pub email: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub connected_at: Option<DateTime<Utc>>,
}

/// Safe summary returned to the Settings UI — never includes the token.
#[derive(Debug, Clone, Serialize)]
pub struct OAuthStatus {
    pub provider: OAuthProvider,
    pub email: String,
    pub scope: String,
    pub connected_at: Option<DateTime<Utc>>,
}

/// Machine-level OAuth client (one per provider), reused by every
/// account. The client id is public; the secret is the developer
/// credential and is kept in the keychain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAppClient {
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
}

/// Safe app-client summary for the wizard — no secret material.
#[derive(Debug, Clone, Serialize)]
pub struct OAuthAppStatus {
    pub client_id: String,
    pub has_secret: bool,
}

pub fn app_client(provider: OAuthProvider) -> Result<Option<OAuthAppClient>, UnkaiError> {
    let Some(raw) = credentials::get_oauth_app(provider.id())? else {
        return Ok(None);
    };
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|e| UnkaiError::Storage(format!("OAuth app credential parse failed: {e}")))
}

pub fn app_client_status(provider: OAuthProvider) -> Result<Option<OAuthAppStatus>, UnkaiError> {
    Ok(app_client(provider)?.map(|client| OAuthAppStatus {
        client_id: client.client_id,
        has_secret: !client.client_secret.trim().is_empty(),
    }))
}

pub fn set_app_client(provider: OAuthProvider, client: &OAuthAppClient) -> Result<(), UnkaiError> {
    let raw = serde_json::to_string(client).map_err(|e| {
        UnkaiError::Storage(format!("OAuth app credential serialization failed: {e}"))
    })?;
    credentials::store_oauth_app(provider.id(), &raw)
}

pub fn clear_app_client(provider: OAuthProvider) -> Result<(), UnkaiError> {
    credentials::delete_oauth_app(provider.id())
}

#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: DateTime<Utc>,
}

fn token_cache() -> &'static Mutex<HashMap<String, CachedToken>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedToken>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default = "default_expires_in")]
    expires_in: u64,
    refresh_token: Option<String>,
    scope: Option<String>,
}

fn default_expires_in() -> u64 {
    3600
}

#[derive(Debug, Deserialize)]
struct OAuthErrorBody {
    error: String,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

type SharedSender = Arc<Mutex<Option<oneshot::Sender<CallbackParams>>>>;

async fn callback(
    State(sender): State<SharedSender>,
    Query(params): Query<CallbackParams>,
) -> Html<&'static str> {
    if let Some(tx) = sender.lock().await.take() {
        let _ = tx.send(params);
    }
    Html(
        "<!doctype html><html><body style=\"font-family:system-ui,sans-serif;background:#121212;color:#d4d4d4;padding:3rem\">\
         <h2>Index Mail</h2><p>Sign-in received. You can close this window and return to the app.</p>\
         </body></html>",
    )
}

struct CallbackServer {
    port: u16,
    handles: Vec<JoinHandle<()>>,
    receiver: Option<oneshot::Receiver<CallbackParams>>,
}

impl CallbackServer {
    fn redirect_uri(&self, host: &str, path: &str) -> String {
        format!("http://{host}:{}{}", self.port, path)
    }

    fn take_receiver(&mut self) -> Option<oneshot::Receiver<CallbackParams>> {
        self.receiver.take()
    }

    fn abort(self) {
        for handle in self.handles {
            handle.abort();
        }
    }
}

async fn start_callback_server(host: &'static str) -> Result<CallbackServer, UnkaiError> {
    let (listener_v4, port) = {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|e| UnkaiError::Network(format!("OAuth callback listener failed: {e}")))?;
        let port = listener
            .local_addr()
            .map_err(|e| UnkaiError::Protocol(format!("OAuth callback address failed: {e}")))?
            .port();
        (listener, port)
    };

    let (tx, rx) = oneshot::channel();
    let sender: SharedSender = Arc::new(Mutex::new(Some(tx)));
    let app = Router::new()
        .route(CALLBACK_PATH, get(callback))
        .route("/", get(callback))
        .with_state(sender);

    let mut handles = Vec::new();
    let app_v4 = app.clone();
    handles.push(tokio::spawn(async move {
        let _ = axum::serve(listener_v4, app_v4).await;
    }));

    // `localhost` can resolve to ::1 first. Serving both families on
    // the same port keeps the browser callback working regardless of
    // which address it picks. If IPv6 is unavailable, IPv4 is enough.
    if host == "localhost"
        && let Ok(listener_v6) = TcpListener::bind(("::1", port)).await
    {
        handles.push(tokio::spawn(async move {
            let _ = axum::serve(listener_v6, app).await;
        }));
    }

    Ok(CallbackServer {
        port,
        handles,
        receiver: Some(rx),
    })
}

fn pkce_verifier() -> String {
    // 64 hex characters, within PKCE's 43–128 character range. UUID v4
    // draws from the OS CSPRNG.
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn build_auth_url(
    provider: OAuthProvider,
    client_id: &str,
    redirect_uri: &str,
    challenge: &str,
    state: &str,
    email: &str,
) -> Result<reqwest::Url, UnkaiError> {
    let mut url = reqwest::Url::parse(provider.auth_url())
        .map_err(|e| UnkaiError::Protocol(format!("invalid OAuth authorize URL: {e}")))?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("client_id", client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &provider.scopes().join(" "))
            .append_pair("code_challenge", challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", state);
        match provider {
            OAuthProvider::Google => {
                q.append_pair("access_type", "offline")
                    .append_pair("prompt", "consent");
            }
            OAuthProvider::Microsoft => {
                q.append_pair("response_mode", "query")
                    .append_pair("prompt", "select_account");
            }
        }
        if !email.trim().is_empty() {
            q.append_pair("login_hint", email.trim());
        }
    }
    Ok(url)
}

async fn token_request(url: &str, params: &[(&str, String)]) -> Result<TokenResponse, UnkaiError> {
    let client = reqwest::Client::builder()
        .timeout(TOKEN_TIMEOUT)
        .build()
        .map_err(|e| UnkaiError::Network(format!("OAuth HTTP client failed: {e}")))?;
    let response = client
        .post(url)
        .form(params)
        .send()
        .await
        .map_err(|e| UnkaiError::Network(format!("OAuth token request failed: {e}")))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| UnkaiError::Network(format!("OAuth token response read failed: {e}")))?;

    if !status.is_success() {
        let detail = serde_json::from_str::<OAuthErrorBody>(&body)
            .map(|e| match e.error_description {
                Some(description) => format!("{}: {description}", e.error),
                None => e.error,
            })
            .unwrap_or_else(|_| format!("HTTP {status}"));
        return Err(UnkaiError::Auth(format!(
            "OAuth token endpoint rejected the request: {detail}"
        )));
    }

    serde_json::from_str(&body)
        .map_err(|e| UnkaiError::Protocol(format!("OAuth token response was not JSON: {e}")))
}

async fn exchange_code(
    provider: OAuthProvider,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<TokenResponse, UnkaiError> {
    let mut params = vec![
        ("client_id", client_id.to_string()),
        ("code", code.to_string()),
        ("redirect_uri", redirect_uri.to_string()),
        ("grant_type", "authorization_code".to_string()),
        ("code_verifier", verifier.to_string()),
    ];
    if !client_secret.trim().is_empty() {
        params.push(("client_secret", client_secret.trim().to_string()));
    }
    if let Some(scope) = provider.token_scope() {
        params.push(("scope", scope.to_string()));
    }
    token_request(provider.token_url(), &params).await
}

async fn refresh_token(credentials: &OAuthCredentials) -> Result<TokenResponse, UnkaiError> {
    let mut params = vec![
        ("client_id", credentials.client_id.clone()),
        ("refresh_token", credentials.refresh_token.clone()),
        ("grant_type", "refresh_token".to_string()),
    ];
    if !credentials.client_secret.trim().is_empty() {
        params.push((
            "client_secret",
            credentials.client_secret.trim().to_string(),
        ));
    }
    if let Some(scope) = credentials.provider.token_scope() {
        params.push(("scope", scope.to_string()));
    }
    token_request(credentials.provider.token_url(), &params).await
}

fn store_credentials(account_id: &str, credentials: &OAuthCredentials) -> Result<(), UnkaiError> {
    let json = serde_json::to_string(credentials)
        .map_err(|e| UnkaiError::Storage(format!("OAuth credential serialization failed: {e}")))?;
    credentials::store_oauth(account_id, &json)
}

fn load_credentials(account_id: &str) -> Result<Option<OAuthCredentials>, UnkaiError> {
    let Some(raw) = credentials::get_oauth(account_id)? else {
        return Ok(None);
    };
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|e| UnkaiError::Storage(format!("OAuth credential parse failed: {e}")))
}

async fn cache_access_token(account_id: &str, access_token: &str, expires_in: u64) {
    let expires_at = Utc::now() + TimeDelta::seconds(expires_in.max(60) as i64);
    token_cache().lock().await.insert(
        account_id.to_string(),
        CachedToken {
            access_token: access_token.to_string(),
            expires_at,
        },
    );
}

/// Whether this account has OAuth2 credentials in the keychain.
pub fn has_credentials(account_id: &str) -> Result<bool, UnkaiError> {
    Ok(credentials::get_oauth(account_id)?.is_some())
}

/// Provider for a connected account, if any.
pub fn provider_for_account(account_id: &str) -> Result<Option<OAuthProvider>, UnkaiError> {
    Ok(load_credentials(account_id)?.map(|c| c.provider))
}

/// Safe status for the Settings UI.
pub fn status(account_id: &str) -> Result<Option<OAuthStatus>, UnkaiError> {
    let Some(credentials) = load_credentials(account_id)? else {
        return Ok(None);
    };
    Ok(Some(OAuthStatus {
        provider: credentials.provider,
        email: credentials.email,
        scope: credentials.scope,
        connected_at: credentials.connected_at,
    }))
}

/// Forget the OAuth2 credentials for an account. The stored mail cache
/// is untouched; this only removes the keychain entry and the in-memory
/// access token.
pub fn disconnect(account_id: &str) -> Result<(), UnkaiError> {
    if let Ok(mut cache) = token_cache().try_lock() {
        cache.remove(account_id);
    }
    credentials::delete_oauth(account_id)
}

/// Return a valid access token, refreshing it first when the cached one
/// is missing or close to expiry.
pub async fn access_token(account_id: &str) -> Result<String, UnkaiError> {
    {
        let cache = token_cache().lock().await;
        if let Some(cached) = cache.get(account_id)
            && cached.expires_at > Utc::now() + TimeDelta::seconds(60)
        {
            return Ok(cached.access_token.clone());
        }
    }

    let mut credentials = load_credentials(account_id)?.ok_or_else(|| {
        UnkaiError::Auth(format!(
            "Google/Microsoft OAuth is not connected for account '{account_id}'"
        ))
    })?;

    let response = refresh_token(&credentials).await.map_err(|e| {
        UnkaiError::Auth(format!(
            "{} OAuth token refresh failed; reconnect the account: {e}",
            credentials.provider.display_name()
        ))
    })?;

    if let Some(new_refresh) = response.refresh_token {
        credentials.refresh_token = new_refresh;
        store_credentials(account_id, &credentials)?;
    }
    if let Some(scope) = response.scope {
        credentials.scope = scope;
    }

    cache_access_token(account_id, &response.access_token, response.expires_in).await;
    Ok(response.access_token)
}

/// Run the browser sign-in flow and persist the refresh token.
///
/// On success the command layer tests both IMAP and SMTP with the new
/// token before returning, so the wizard never saves an account that
/// signed in but cannot read or send mail.
#[allow(clippy::too_many_arguments)]
pub async fn authorize(
    provider: OAuthProvider,
    account_id: &str,
    client_id: &str,
    client_secret: &str,
    email: &str,
    imap_host: &str,
    imap_port: u16,
    smtp_host: &str,
    smtp_port: u16,
    trusted_certs: &[unkai_core::models::TrustedCert],
    remember_client: bool,
) -> Result<(), UnkaiError> {
    // Empty client fields mean "use the one already saved for this
    // installation". That is the normal path after the first account:
    // the user just clicks Sign in, like any other mail client.
    let (effective_client_id, effective_client_secret) = if client_id.trim().is_empty() {
        let saved = app_client(provider)?.ok_or_else(|| {
            UnkaiError::Auth(
                "No OAuth client is configured yet — create a desktop OAuth app in the provider's developer console first".into(),
            )
        })?;
        (saved.client_id, saved.client_secret)
    } else {
        (
            client_id.trim().to_string(),
            client_secret.trim().to_string(),
        )
    };
    if provider == OAuthProvider::Google && effective_client_secret.trim().is_empty() {
        return Err(UnkaiError::Auth(
            "Google desktop OAuth requires the client secret from the same OAuth client".into(),
        ));
    }
    if email.trim().is_empty() || imap_host.trim().is_empty() || smtp_host.trim().is_empty() {
        return Err(UnkaiError::Protocol(
            "OAuth sign-in needs the email address and both mail server hosts".into(),
        ));
    }

    let verifier = pkce_verifier();
    let challenge = pkce_challenge(&verifier);
    let state = Uuid::new_v4().simple().to_string();
    let mut server = start_callback_server(provider.redirect_host()).await?;
    let redirect_uri = server.redirect_uri(provider.redirect_host(), provider.callback_path());
    let auth_url = build_auth_url(
        provider,
        &effective_client_id,
        &redirect_uri,
        &challenge,
        &state,
        email,
    )?;

    if let Err(e) = open::that(auth_url.as_str()) {
        server.abort();
        return Err(UnkaiError::Network(format!(
            "could not open the browser for OAuth sign-in: {e}"
        )));
    }

    let receiver = server.take_receiver().ok_or_else(|| {
        UnkaiError::Protocol("OAuth callback receiver was already consumed".into())
    })?;
    let params = match tokio::time::timeout(CALLBACK_TIMEOUT, receiver).await {
        Ok(Ok(params)) => params,
        Ok(Err(_)) => {
            server.abort();
            return Err(UnkaiError::Auth(
                "OAuth callback listener closed before sign-in completed".into(),
            ));
        }
        Err(_) => {
            server.abort();
            return Err(UnkaiError::Auth(
                "OAuth sign-in timed out — no browser callback arrived".into(),
            ));
        }
    };
    server.abort();

    if let Some(error) = params.error {
        let detail = params.error_description.unwrap_or(error);
        return Err(UnkaiError::Auth(format!(
            "OAuth sign-in was cancelled or rejected: {detail}"
        )));
    }
    if params.state.as_deref() != Some(state.as_str()) {
        return Err(UnkaiError::Auth(
            "OAuth callback state mismatch — sign-in was rejected".into(),
        ));
    }
    let code = params
        .code
        .ok_or_else(|| UnkaiError::Auth("OAuth callback contained no authorization code".into()))?;

    let response = exchange_code(
        provider,
        &effective_client_id,
        &effective_client_secret,
        &code,
        &redirect_uri,
        &verifier,
    )
    .await?;
    let refresh_token = response.refresh_token.ok_or_else(|| {
        UnkaiError::Auth(
            "OAuth provider did not return a refresh token; revoke the app's access and try again"
                .into(),
        )
    })?;

    let credentials = OAuthCredentials {
        provider,
        client_id: effective_client_id.clone(),
        client_secret: effective_client_secret.clone(),
        refresh_token,
        email: email.trim().to_string(),
        scope: response
            .scope
            .unwrap_or_else(|| provider.scopes().join(" ")),
        connected_at: Some(Utc::now()),
    };
    store_credentials(account_id, &credentials)?;
    cache_access_token(account_id, &response.access_token, response.expires_in).await;

    // A successful token exchange is not enough: prove the token can
    // actually open the mailbox and submit mail. This also surfaces
    // tenant policies that disable IMAP/SMTP.
    let imap = ImapClient::connect_oauth(
        imap_host.trim(),
        imap_port,
        email.trim(),
        &response.access_token,
        trusted_certs,
    )
    .await?;
    let _ = imap.logout().await;

    let _smtp = SmtpClient::connect_oauth(
        smtp_host.trim(),
        smtp_port,
        email.trim(),
        &response.access_token,
        trusted_certs,
    )
    .await?;

    if remember_client && !client_id.trim().is_empty() {
        let app = OAuthAppClient {
            client_id: effective_client_id,
            client_secret: effective_client_secret,
        };
        set_app_client(provider, &app)?;
        tracing::info!(
            provider = provider.id(),
            "Saved OAuth app client for reuse by future accounts"
        );
    }

    tracing::info!(
        account_id,
        provider = provider.id(),
        "OAuth2 sign-in completed and IMAP/SMTP tested"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_matches_rfc7636_vector() {
        // RFC 7636 Appendix B.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            pkce_challenge(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn google_auth_url_has_scope_redirect_and_pkce() {
        let url = build_auth_url(
            OAuthProvider::Google,
            "client.apps.googleusercontent.com",
            "http://127.0.0.1:4321/oauth2/callback",
            "challenge",
            "state123",
            "user@gmail.com",
        )
        .expect("url");
        let pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(pairs.get("scope").map(String::as_str), Some(GOOGLE_SCOPE));
        assert_eq!(
            pairs.get("code_challenge_method").map(String::as_str),
            Some("S256")
        );
        assert_eq!(
            pairs.get("access_type").map(String::as_str),
            Some("offline")
        );
        assert_eq!(pairs.get("state").map(String::as_str), Some("state123"));
        assert_eq!(
            pairs.get("redirect_uri").map(String::as_str),
            Some("http://127.0.0.1:4321/oauth2/callback")
        );
    }

    #[test]
    fn microsoft_auth_url_has_imap_and_smtp_scopes() {
        let url = build_auth_url(
            OAuthProvider::Microsoft,
            "client-id",
            "http://localhost:4321/oauth2/callback",
            "challenge",
            "state123",
            "user@hotmail.com",
        )
        .expect("url");
        let pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
        let scope = pairs.get("scope").expect("scope");
        assert!(scope.contains(MICROSOFT_IMAP_SCOPE));
        assert!(scope.contains(MICROSOFT_SMTP_SCOPE));
        assert!(scope.contains("offline_access"));
        assert_eq!(
            pairs.get("response_mode").map(String::as_str),
            Some("query")
        );
    }

    #[test]
    fn provider_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&OAuthProvider::Microsoft).unwrap(),
            "\"microsoft\""
        );
    }

    #[test]
    fn callback_paths_match_provider_registration_rules() {
        assert_eq!(OAuthProvider::Google.callback_path(), "/");
        assert_eq!(OAuthProvider::Microsoft.callback_path(), "/");
    }

    #[tokio::test]
    async fn callback_server_delivers_query_params() {
        let mut server = start_callback_server("127.0.0.1")
            .await
            .expect("callback server");
        let receiver = server.take_receiver().expect("receiver");
        let url = format!(
            "http://127.0.0.1:{}/oauth2/callback?code=test-code&state=test-state",
            server.port
        );
        let response = reqwest::get(url).await.expect("callback request");
        assert!(response.status().is_success());
        let params = receiver.await.expect("callback payload");
        assert_eq!(params.code.as_deref(), Some("test-code"));
        assert_eq!(params.state.as_deref(), Some("test-state"));
        server.abort();
    }

    #[tokio::test]
    async fn token_request_parses_the_provider_response() {
        use axum::{Json, routing::post};

        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("listener");
        let port = listener.local_addr().expect("addr").port();
        let app = Router::new().route(
            "/token",
            post(|| async {
                Json(serde_json::json!({
                    "access_token": "access-123",
                    "expires_in": 3600,
                    "refresh_token": "refresh-456",
                    "scope": GOOGLE_SCOPE
                }))
            }),
        );
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let response = token_request(
            &format!("http://127.0.0.1:{port}/token"),
            &[("grant_type", "authorization_code".to_string())],
        )
        .await
        .expect("token response");
        assert_eq!(response.access_token, "access-123");
        assert_eq!(response.refresh_token.as_deref(), Some("refresh-456"));
        assert_eq!(response.expires_in, 3600);
        handle.abort();
    }
}
