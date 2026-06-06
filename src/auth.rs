//! Citrate Studio — authentication scaffold (STUDIO-2).
//!
//! Pure, compiling, unit-tested foundation for the OIDC + SIWE flow against
//! `citrate-identity` (`auth.citrate.ai`). This module is deliberately
//! network-free: PKCE, the authorize URL, the token model, claim parsing,
//! token storage, and the wallet→roster mapping. The loopback listener,
//! token exchange, JWKS validation, and OS-keyring storage land in a later,
//! separately-reviewed STUDIO-2 commit (per ADR-2026-06-04-auth-oidc-siwe),
//! so the trust-bearing network code is auditable as one diff.
//!
//! Trust boundary: auth establishes *who the operator is* (a wallet address).
//! It does NOT establish *what they may approve* — that stays with the core's
//! `SignerRoster` + `Quorum`. Auth keys ≠ signer keys.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

/// OIDC client configuration for the native (loopback PKCE) flow.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub issuer: String,     // e.g. https://auth.citrate.ai
    pub client_id: String,  // e.g. citrate-studio (see Open Question Q2)
    pub scope: String,      // "openid profile wallet kyc offline_access"
    pub redirect_uri: String, // http://127.0.0.1:<port>/auth/callback (RFC 8252)
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            issuer: "https://auth.citrate.ai".into(),
            client_id: "citrate-studio".into(),
            scope: "openid profile wallet kyc offline_access".into(),
            redirect_uri: "http://127.0.0.1:0/auth/callback".into(),
        }
    }
}

impl AuthConfig {
    /// The §3.1.3.7 JWKS-skip is only sound over a TLS-authenticated channel: we trust the
    /// ID token *because* it arrived directly from the token endpoint over server-validated
    /// TLS, not because we checked its signature. Enforce that precondition — the issuer must
    /// be `https://`. A loopback dev issuer (`http://127.0.0.1` / `localhost` / `[::1]`) is
    /// permitted ONLY in debug builds, and loudly; a release build rejects any non-https
    /// issuer, fail-closed. (STUDIO-16 / audit F-2.)
    pub fn validate_issuer(&self) -> Result<(), AuthError> {
        let iss = self.issuer.trim_end_matches('/');
        if iss.starts_with("https://") {
            return Ok(());
        }
        #[cfg(debug_assertions)]
        {
            let loopback = iss.starts_with("http://127.0.0.1")
                || iss.starts_with("http://localhost")
                || iss.starts_with("http://[::1]");
            if loopback {
                eprintln!(
                    "WARN: INSECURE plaintext OIDC issuer {iss} — permitted in debug builds \
                     only; the ID token is trusted without a TLS-authenticated channel. \
                     Never ship this."
                );
                return Ok(());
            }
        }
        Err(AuthError::Claims(format!(
            "insecure issuer scheme (must be https): {iss}"
        )))
    }
}

/// A PKCE (RFC 7636) verifier/challenge pair using the S256 method that
/// `citrate-identity` requires.
#[derive(Debug, Clone)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    /// Build from a caller-supplied verifier (the real flow generates the
    /// verifier from `OsRng`; tests use the RFC 7636 vector).
    pub fn from_verifier(verifier: impl Into<String>) -> Self {
        let verifier = verifier.into();
        let digest = Sha256::digest(verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(digest);
        Self { verifier, challenge }
    }

    /// Build from 32 bytes of entropy: verifier = base64url(entropy),
    /// challenge = base64url(sha256(verifier)).
    pub fn from_entropy(entropy: &[u8; 32]) -> Self {
        Self::from_verifier(URL_SAFE_NO_PAD.encode(entropy))
    }
}

/// Minimal percent-encoding for query-parameter values (RFC 3986 unreserved
/// kept; everything else escaped). Enough for the authorize URL; the network
/// commit will use `url::Url` proper.
fn pct(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Build the `/auth` authorization-code URL (PKCE S256).
pub fn authorize_url(cfg: &AuthConfig, pkce: &Pkce, state: &str) -> String {
    format!(
        "{issuer}/auth?response_type=code&client_id={cid}&redirect_uri={ru}\
         &scope={scope}&code_challenge={cc}&code_challenge_method=S256&state={state}",
        issuer = cfg.issuer.trim_end_matches('/'),
        cid = pct(&cfg.client_id),
        ru = pct(&cfg.redirect_uri),
        scope = pct(&cfg.scope),
        cc = pct(&pkce.challenge),
        state = pct(state),
    )
}

/// Tokens returned from `POST /token` (id + access + optional refresh).
#[derive(Debug, Clone, Default)]
pub struct TokenSet {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64, // unix seconds; 0 = unknown
}

impl TokenSet {
    pub fn to_json(&self) -> String {
        serde_json::json!({
            "id_token": self.id_token,
            "access_token": self.access_token,
            "refresh_token": self.refresh_token,
            "expires_at": self.expires_at,
        })
        .to_string()
    }
    pub fn from_json(s: &str) -> Option<TokenSet> {
        let v: serde_json::Value = serde_json::from_str(s).ok()?;
        Some(TokenSet {
            id_token: v.get("id_token")?.as_str()?.to_string(),
            access_token: v.get("access_token").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            refresh_token: v.get("refresh_token").and_then(|x| x.as_str()).map(|s| s.to_string()),
            expires_at: v.get("expires_at").and_then(|x| x.as_u64()).unwrap_or(0),
        })
    }
}

/// Claims we care about, extracted from the ID token. In `citrate-identity`,
/// `sub` == `wallet_address` == an EIP-55 address.
#[derive(Debug, Clone, Default)]
pub struct Claims {
    pub wallet_address: String, // == the `sub` claim (EIP-55 address)
    pub kyc_status: Option<String>,
    pub iss: String,
    pub audiences: Vec<String>,  // all audiences (OIDC `aud` may be string or array)
    pub azp: Option<String>,     // authorized party — required when multi-audience
    pub exp: i64,
}

/// Decode the ID-token payload. For the native code flow the token is received
/// directly from the `/token` endpoint over a TLS channel we initiated, so per
/// OIDC Core §3.1.3.7 it is trusted via TLS server authentication — we do NOT
/// re-verify the RS256 signature (no jsonwebtoken/JWKS). `validate_claims`
/// then checks iss/aud/exp as defense in depth.
pub fn decode_claims(id_token: &str) -> Option<Claims> {
    let payload_b64 = id_token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(|x| x.to_string());
    // `aud` may be a string or an array; collect ALL audiences so validation can check
    // membership (not just the first element) and enforce `azp` for the multi-aud case.
    let audiences: Vec<String> = match v.get("aud") {
        Some(serde_json::Value::String(s)) => vec![s.clone()],
        Some(serde_json::Value::Array(arr)) => {
            arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()
        }
        _ => Vec::new(),
    };
    Some(Claims {
        wallet_address: s("wallet_address").or_else(|| s("sub")).unwrap_or_default(),
        kyc_status: s("kyc_status"),
        iss: s("iss").unwrap_or_default(),
        audiences,
        azp: s("azp"),
        exp: v.get("exp").and_then(|x| x.as_i64()).unwrap_or(0),
    })
}

/// Validate the trusted ID-token claims: issuer + audience match our config,
/// and the token has not expired.
pub fn validate_claims(c: &Claims, cfg: &AuthConfig, now_unix: i64) -> Result<(), AuthError> {
    // Issuer must be configured and must match — an empty configured issuer is rejected
    // (no silent skip), so the iss check is always load-bearing. (STUDIO-16 / audit F-2.)
    if cfg.issuer.is_empty() {
        return Err(AuthError::Claims("no configured issuer".into()));
    }
    if c.iss != cfg.issuer {
        return Err(AuthError::Claims(format!("issuer mismatch: {}", c.iss)));
    }
    // Audience: our client_id must be a MEMBER of `aud` (not merely the first element).
    if !c.audiences.iter().any(|a| a == &cfg.client_id) {
        return Err(AuthError::Claims(format!("audience mismatch: {:?}", c.audiences)));
    }
    // OIDC Core §3.1.3.7: when there are multiple audiences, `azp` MUST be present and equal
    // our client_id.
    if c.audiences.len() > 1 && c.azp.as_deref() != Some(cfg.client_id.as_str()) {
        return Err(AuthError::Claims("multiple audiences without matching azp".into()));
    }
    if c.exp != 0 && c.exp <= now_unix {
        return Err(AuthError::Claims("id_token expired".into()));
    }
    if c.wallet_address.is_empty() {
        return Err(AuthError::Claims("no wallet_address claim".into()));
    }
    Ok(())
}

/// Where session tokens are persisted. Production is `KeyringTokenStore` (OS
/// keyring: macOS Keychain / Windows Credential Manager / libsecret); a
/// plaintext `FileTokenStore` exists for tests only.
pub trait TokenStore {
    fn load(&self) -> Option<TokenSet>;
    fn save(&self, tokens: &TokenSet) -> std::io::Result<()>;
    fn clear(&self) -> std::io::Result<()>;
}

/// Test-only token store — plaintext JSON on disk, so the flow is testable
/// without touching the OS keyring. Production uses `KeyringTokenStore`.
#[cfg(test)]
pub struct FileTokenStore {
    pub path: std::path::PathBuf,
}

#[cfg(test)]
impl TokenStore for FileTokenStore {
    fn load(&self) -> Option<TokenSet> {
        TokenSet::from_json(&std::fs::read_to_string(&self.path).ok()?)
    }
    fn save(&self, t: &TokenSet) -> std::io::Result<()> {
        std::fs::write(&self.path, t.to_json())
    }
    fn clear(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

/// Production token store — the OS keyring (macOS Keychain / Windows Credential
/// Manager / libsecret). Tokens never touch plaintext disk.
pub struct KeyringTokenStore {
    pub service: String,
    pub account: String,
}
impl KeyringTokenStore {
    pub fn citrate() -> Self {
        Self { service: "citrate-studio".into(), account: "oidc-tokens".into() }
    }
    fn entry(&self) -> Option<keyring::Entry> {
        keyring::Entry::new(&self.service, &self.account).ok()
    }
}
impl TokenStore for KeyringTokenStore {
    fn load(&self) -> Option<TokenSet> {
        TokenSet::from_json(&self.entry()?.get_password().ok()?)
    }
    fn save(&self, t: &TokenSet) -> std::io::Result<()> {
        self.entry()
            .ok_or_else(|| std::io::Error::other("keyring entry"))?
            .set_password(&t.to_json())
            .map_err(std::io::Error::other)
    }
    fn clear(&self) -> std::io::Result<()> {
        if let Some(e) = self.entry() {
            let _ = e.delete_password();
        }
        Ok(())
    }
}

// Identity → signer-roster enrollment lives in `signing.rs` (STUDIO-4):
// `signing::signer_id`, `signing::Roster`. The earlier auth-side scaffold was
// superseded and removed in the STUDIO-7 hardening pass.

// =============================================================
// Native loopback-PKCE login flow (RFC 8252).
// =============================================================

/// Errors from the login flow.
#[derive(Debug)]
pub enum AuthError {
    Io(String),
    Http(String),
    State,
    Token(String),
    Claims(String),
    Browser(String),
}
impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Io(s) => write!(f, "io: {s}"),
            AuthError::Http(s) => write!(f, "could not reach the identity service: {s}"),
            AuthError::State => write!(f, "state mismatch (possible CSRF) — sign-in aborted"),
            AuthError::Token(s) => write!(f, "token error: {s}"),
            AuthError::Claims(s) => write!(f, "id_token invalid: {s}"),
            AuthError::Browser(s) => write!(f, "could not open the browser: {s}"),
        }
    }
}
impl std::error::Error for AuthError {}

/// The resolved session after a successful login.
#[derive(Debug, Clone, Default)]
pub struct AuthSession {
    pub wallet: String,
    pub kyc: String,
}

fn random_bytes() -> [u8; 32] {
    let mut b = [0u8; 32];
    getrandom::getrandom(&mut b).expect("OS entropy");
    b
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Open `url` in the system browser (no dependency — platform launcher).
pub fn open_url(url: &str) -> Result<(), AuthError> {
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    cmd.spawn().map(|_| ()).map_err(|e| AuthError::Browser(e.to_string()))
}

/// Minimal percent-decode for callback query values.
fn pct_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => { out.push(b' '); i += 1; }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => { out.push(b); i += 3; }
                    Err(_) => { out.push(bytes[i]); i += 1; }
                }
            }
            b => { out.push(b); i += 1; }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Parse a `path?key=val&...` query into a map (decoded).
fn parse_query(path: &str) -> std::collections::HashMap<String, String> {
    let q = path.split_once('?').map(|x| x.1).unwrap_or("");
    q.split('&')
        .filter_map(|kv| {
            let mut it = kv.splitn(2, '=');
            let k = it.next()?;
            let v = it.next().unwrap_or("");
            Some((pct_decode(k), pct_decode(v)))
        })
        .collect()
}

/// Block on the loopback listener until the OAuth callback arrives; return the
/// authorization code after checking `state`. Responds to the browser with a
/// close-this-window page.
pub fn wait_for_callback(server: &tiny_http::Server, expected_state: &str) -> Result<String, AuthError> {
    const PAGE: &str = "<!doctype html><meta charset=utf-8><title>Citrate Studio</title>\
        <body style=\"font-family:system-ui;background:#0f2a1a;color:#cde7d6;display:grid;place-items:center;height:100vh;margin:0\">\
        <div style=\"text-align:center\"><h2 style=\"color:#8ecc09\">Signed in to Citrate Studio</h2>\
        <p>You can close this window and return to the app.</p></div>";
    for req in server.incoming_requests() {
        let url = req.url().to_string();
        if !url.starts_with("/auth/callback") {
            let _ = req.respond(tiny_http::Response::from_string("not found").with_status_code(404));
            continue;
        }
        let q = parse_query(&url);
        let header = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
        let _ = req.respond(tiny_http::Response::from_string(PAGE).with_header(header));
        if let Some(err) = q.get("error") {
            return Err(AuthError::Token(err.clone()));
        }
        match (q.get("code"), q.get("state")) {
            (Some(code), Some(state)) if state == expected_state => return Ok(code.clone()),
            (_, Some(_)) => return Err(AuthError::State),
            _ => return Err(AuthError::Token("no code in callback".into())),
        }
    }
    Err(AuthError::Io("loopback listener closed".into()))
}

/// Parse the `POST /token` JSON response into a `TokenSet`.
pub fn parse_token_response(json: &str) -> Result<TokenSet, AuthError> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| AuthError::Token(e.to_string()))?;
    let g = |k: &str| v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string());
    let id_token = g("id_token").ok_or_else(|| AuthError::Token("no id_token in response".into()))?;
    let expires_in = v.get("expires_in").and_then(|x| x.as_u64()).unwrap_or(0);
    Ok(TokenSet {
        id_token,
        access_token: g("access_token").unwrap_or_default(),
        refresh_token: g("refresh_token"),
        expires_at: if expires_in > 0 { now_unix() as u64 + expires_in } else { 0 },
    })
}

/// Exchange the authorization code for tokens at `{issuer}/token` (PKCE).
pub fn exchange_code(cfg: &AuthConfig, redirect: &str, code: &str, verifier: &str) -> Result<TokenSet, AuthError> {
    let url = format!("{}/token", cfg.issuer.trim_end_matches('/'));
    let resp = ureq::post(&url)
        .send_form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", &cfg.client_id),
            ("redirect_uri", redirect),
            ("code_verifier", verifier),
        ])
        .map_err(|e| AuthError::Http(e.to_string()))?;
    let body = resp.into_string().map_err(|e| AuthError::Http(e.to_string()))?;
    parse_token_response(&body)
}

/// Run the full native loopback-PKCE login: bind an ephemeral 127.0.0.1 port,
/// open the browser to the authorize URL, capture the callback, exchange the
/// code, validate the (TLS-trusted) ID token, and persist the tokens.
/// Blocking — call off the UI thread.
pub fn login(base: &AuthConfig, store: &dyn TokenStore) -> Result<AuthSession, AuthError> {
    base.validate_issuer()?; // fail-closed before opening a browser to a plaintext issuer
    let server = tiny_http::Server::http("127.0.0.1:0").map_err(|e| AuthError::Io(e.to_string()))?;
    let port = server
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .ok_or_else(|| AuthError::Io("no loopback port".into()))?;
    let redirect = format!("http://127.0.0.1:{port}/auth/callback");
    let cfg = AuthConfig { redirect_uri: redirect.clone(), ..base.clone() };

    let pkce = Pkce::from_entropy(&random_bytes());
    let state = URL_SAFE_NO_PAD.encode(random_bytes());
    open_url(&authorize_url(&cfg, &pkce, &state))?;

    let code = wait_for_callback(&server, &state)?;
    let tokens = exchange_code(&cfg, &redirect, &code, &pkce.verifier)?;
    let claims = decode_claims(&tokens.id_token).ok_or_else(|| AuthError::Claims("undecodable id_token".into()))?;
    validate_claims(&claims, &cfg, now_unix())?;
    store.save(&tokens).map_err(|e| AuthError::Io(e.to_string()))?;
    Ok(AuthSession { wallet: claims.wallet_address, kyc: claims.kyc_status.unwrap_or_default() })
}

/// Server-side revoke (best-effort) + local token clear.
pub fn logout(cfg: &AuthConfig, store: &dyn TokenStore) -> Result<(), AuthError> {
    if let Some(t) = store.load() {
        if !t.access_token.is_empty() {
            let url = format!("{}/logout", cfg.issuer.trim_end_matches('/'));
            let _ = ureq::post(&url)
                .set("Authorization", &format!("Bearer {}", t.access_token))
                .call();
        }
    }
    store.clear().map_err(|e| AuthError::Io(e.to_string()))
}

/// Exchange the stored refresh token for fresh tokens (rotating refresh).
pub fn refresh(cfg: &AuthConfig, store: &dyn TokenStore) -> Result<TokenSet, AuthError> {
    cfg.validate_issuer()?; // never refresh a trusted token over a plaintext channel
    let cur = store.load().ok_or_else(|| AuthError::Token("no stored tokens".into()))?;
    let rt = cur.refresh_token.ok_or_else(|| AuthError::Token("no refresh_token".into()))?;
    let url = format!("{}/token", cfg.issuer.trim_end_matches('/'));
    let resp = ureq::post(&url)
        .send_form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt),
            ("client_id", &cfg.client_id),
        ])
        .map_err(|e| AuthError::Http(e.to_string()))?;
    let body = resp.into_string().map_err(|e| AuthError::Http(e.to_string()))?;
    let tokens = parse_token_response(&body)?;
    store.save(&tokens).map_err(|e| AuthError::Io(e.to_string()))?;
    Ok(tokens)
}

/// The current session from stored tokens (for app startup). Validates the ID
/// token; on expiry, attempts a single refresh before giving up.
pub fn session_from_store(cfg: &AuthConfig, store: &dyn TokenStore) -> Option<AuthSession> {
    let to_session = |c: Claims| AuthSession { wallet: c.wallet_address, kyc: c.kyc_status.unwrap_or_default() };
    let t = store.load()?;
    if let Some(c) = decode_claims(&t.id_token) {
        if validate_claims(&c, cfg, now_unix()).is_ok() {
            return Some(to_session(c));
        }
    }
    // expired / invalid → try one refresh
    let t2 = refresh(cfg, store).ok()?;
    let c2 = decode_claims(&t2.id_token)?;
    validate_claims(&c2, cfg, now_unix()).ok()?;
    Some(to_session(c2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_s256_rfc7636_vector() {
        // RFC 7636 Appendix B.
        let p = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
        assert_eq!(p.challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn pkce_from_entropy_is_url_safe() {
        let p = Pkce::from_entropy(&[7u8; 32]);
        assert!(p.verifier.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert!(!p.challenge.contains('='));
    }

    #[test]
    fn authorize_url_has_required_params() {
        let cfg = AuthConfig::default();
        let p = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
        let url = authorize_url(&cfg, &p, "xyz-state");
        for needle in [
            "/auth?response_type=code",
            "client_id=citrate-studio",
            "code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
            "code_challenge_method=S256",
            "state=xyz-state",
        ] {
            assert!(url.contains(needle), "missing {needle} in {url}");
        }
        // redirect + scope are percent-encoded
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1"));
        assert!(url.contains("openid%20profile%20wallet"));
    }

    #[test]
    fn decode_and_validate_claims() {
        let payload = URL_SAFE_NO_PAD.encode(
            br#"{"sub":"0xAbC","wallet_address":"0xAbC","kyc_status":"verified","iss":"https://auth.citrate.ai","aud":"citrate-studio","exp":9999999999}"#,
        );
        let jwt = format!("eyJhbGciOiJSUzI1NiJ9.{payload}.sig");
        let c = decode_claims(&jwt).unwrap();
        assert_eq!(c.wallet_address, "0xAbC");
        assert_eq!(c.kyc_status.as_deref(), Some("verified"));
        let cfg = AuthConfig::default();
        // valid in 2026
        assert!(validate_claims(&c, &cfg, 1_780_000_000).is_ok());
        // expired
        assert!(validate_claims(&c, &cfg, 10_000_000_000).is_err());
        // wrong audience
        let mut bad = c.clone();
        bad.audiences = vec!["citrate-explorer".into()];
        assert!(validate_claims(&bad, &cfg, 1_780_000_000).is_err());
    }

    #[test]
    fn aud_array_membership_and_azp() {
        // STUDIO-16 / F-2: validate audience by membership, with azp for multi-aud.
        let cfg = AuthConfig::default(); // client_id = citrate-studio
        let mk = |aud: serde_json::Value, azp: Option<&str>| {
            let mut obj = serde_json::json!({
                "sub":"0xAbC","wallet_address":"0xAbC",
                "iss":"https://auth.citrate.ai","exp":9_999_999_999i64
            });
            obj["aud"] = aud;
            if let Some(a) = azp { obj["azp"] = serde_json::json!(a); }
            let payload = URL_SAFE_NO_PAD.encode(obj.to_string().as_bytes());
            decode_claims(&format!("h.{payload}.sig")).unwrap()
        };
        let now = 1_780_000_000;
        // client_id present but NOT first → valid (membership, not position).
        assert!(validate_claims(&mk(serde_json::json!(["other", "citrate-studio"]), Some("citrate-studio")), &cfg, now).is_ok());
        // client_id absent → reject.
        assert!(validate_claims(&mk(serde_json::json!(["other"]), None), &cfg, now).is_err());
        // multiple audiences without matching azp → reject.
        assert!(validate_claims(&mk(serde_json::json!(["citrate-studio", "other"]), None), &cfg, now).is_err());
        assert!(validate_claims(&mk(serde_json::json!(["citrate-studio", "other"]), Some("other")), &cfg, now).is_err());
        // single audience needs no azp.
        assert!(validate_claims(&mk(serde_json::json!("citrate-studio"), None), &cfg, now).is_ok());
    }

    #[test]
    fn empty_issuer_is_rejected() {
        let c = Claims {
            iss: "".into(),
            audiences: vec!["citrate-studio".into()],
            wallet_address: "0x".into(),
            ..Default::default()
        };
        let cfg = AuthConfig { issuer: "".into(), ..AuthConfig::default() };
        assert!(validate_claims(&c, &cfg, 1).is_err(), "empty issuer must not skip the iss check");
    }

    #[test]
    fn issuer_scheme_enforced() {
        // https always ok.
        assert!(AuthConfig { issuer: "https://auth.citrate.ai".into(), ..AuthConfig::default() }.validate_issuer().is_ok());
        // non-loopback http rejected even in debug.
        assert!(AuthConfig { issuer: "http://evil.example".into(), ..AuthConfig::default() }.validate_issuer().is_err());
        // loopback http allowed only in debug (tests run debug); release rejects it.
        #[cfg(debug_assertions)]
        assert!(AuthConfig { issuer: "http://127.0.0.1:3000".into(), ..AuthConfig::default() }.validate_issuer().is_ok());
    }

    #[test]
    fn file_token_store_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("citrate-studio-test-tokens-{}.json", std::process::id()));
        let store = FileTokenStore { path: path.clone() };
        let t = TokenSet { id_token: "id".into(), access_token: "ac".into(), refresh_token: Some("rf".into()), expires_at: 42 };
        store.save(&t).unwrap();
        let back = store.load().unwrap();
        assert_eq!(back.id_token, "id");
        assert_eq!(back.refresh_token.as_deref(), Some("rf"));
        assert_eq!(back.expires_at, 42);
        store.clear().unwrap();
        assert!(store.load().is_none());
    }

    #[test]
    fn query_parse_decodes() {
        let q = parse_query("/auth/callback?code=a%2Bb&state=xyz-1");
        assert_eq!(q.get("code").unwrap(), "a+b");
        assert_eq!(q.get("state").unwrap(), "xyz-1");
    }

    #[test]
    fn token_response_parses() {
        let t = parse_token_response(
            r#"{"id_token":"h.p.s","access_token":"at","refresh_token":"rt","expires_in":3600}"#,
        )
        .unwrap();
        assert_eq!(t.id_token, "h.p.s");
        assert_eq!(t.access_token, "at");
        assert_eq!(t.refresh_token.as_deref(), Some("rt"));
        assert!(t.expires_at > 0);
        assert!(parse_token_response(r#"{"error":"invalid_grant"}"#).is_err());
    }

    #[test]
    fn loopback_capture_and_token_exchange_against_a_mock() {
        use std::io::Write;
        // 1) The OAuth callback: start the loopback listener, fire a callback at
        //    it from a thread, and assert we capture the code + check state.
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let port = server.server_addr().to_ip().unwrap().port();
        let h = std::thread::spawn(move || wait_for_callback(&server, "STATE").map_err(|e| e.to_string()));
        // tiny raw GET so we don't depend on a client lib in the test
        let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(s, "GET /auth/callback?code=THECODE&state=STATE HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").unwrap();
        assert_eq!(h.join().unwrap().unwrap(), "THECODE");

        // 2) The token exchange: stand up a mock /token server and point
        //    exchange_code at it (exercises the ureq POST + JSON parse).
        let token_srv = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let tport = token_srv.server_addr().to_ip().unwrap().port();
        let th = std::thread::spawn(move || {
            if let Some(req) = token_srv.incoming_requests().next() {
                let body = r#"{"id_token":"h.p.s","access_token":"at","token_type":"Bearer","expires_in":3600}"#;
                let hdr = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                let _ = req.respond(tiny_http::Response::from_string(body).with_header(hdr));
            }
        });
        let cfg = AuthConfig { issuer: format!("http://127.0.0.1:{tport}"), ..AuthConfig::default() };
        let tokens = exchange_code(&cfg, "http://127.0.0.1:1/auth/callback", "THECODE", "verifier").unwrap();
        assert_eq!(tokens.id_token, "h.p.s");
        assert_eq!(tokens.access_token, "at");
        th.join().unwrap();
    }
}
