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
        // CIT-STUDIO-02: the issuer flows into the authorize URL that `open_url`
        // hands to the platform browser launcher. Reject any shell/URL metacharacter
        // or whitespace so a hostile issuer value cannot smuggle an argument or
        // command through a launcher that re-parses its string (e.g. Windows `cmd`).
        if let Some(bad) = iss
            .chars()
            .find(|c| c.is_whitespace() || c.is_control() || "\"'`&|;<>^\\".contains(*c))
        {
            return Err(AuthError::Claims(format!(
                "issuer contains a forbidden character {bad:?}: {iss}"
            )));
        }
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

/// The centralized access-entitlement claim every Citrate RP reads — the same
/// `https://citrate.ai/entitlement` contract as `@citrate/oidc-client` and the web
/// RPs (AUTHSPINE S2-WP5). Carried on the `openid` scope; absent ⇒ Public (fail-safe).
pub const ENTITLEMENT_CLAIM: &str = "https://citrate.ai/entitlement";

/// Centralized RBAC entitlement (tier + optional Citrate role) parsed from the
/// entitlement claim. Mirrors the TS `Entitlement` shape exactly.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Entitlement {
    pub tier: String,             // public | commercial | commercial.kyc | academic | confidential
    pub org_id: Option<String>,
    pub citrate_role: Option<String>,
    pub milestone: Option<String>,
    pub expires_at: Option<i64>,  // unix seconds; None = no expiry
}

/// The tier ladder rank, matching TS `TIER_ORDER`. Unknown/absent ⇒ public (0).
pub fn tier_rank(tier: &str) -> u8 {
    match tier {
        "commercial" => 1,
        "commercial.kyc" => 2,
        "academic" => 3,
        "confidential" => 4,
        _ => 0, // public / unknown — fail-safe to the lowest tier
    }
}

/// Parse the entitlement claim object. An unknown/absent tier ⇒ None (Public,
/// fail-safe) — identical to the TS `parseEntitlement` contract.
fn parse_entitlement(v: &serde_json::Value) -> Option<Entitlement> {
    let o = v.as_object()?;
    let tier = o.get("tier").and_then(|x| x.as_str())?;
    if tier_rank(tier) == 0 && tier != "public" {
        return None; // unknown tier string ⇒ treat as no entitlement
    }
    let s = |k: &str| o.get(k).and_then(|x| x.as_str()).map(|x| x.to_string());
    Some(Entitlement {
        tier: tier.to_string(),
        org_id: s("orgId"),
        citrate_role: s("citrateRole"),
        milestone: s("milestone"),
        expires_at: o.get("expiresAt").and_then(|x| x.as_i64()),
    })
}

impl Entitlement {
    /// Effective tier honoring expiry (expired ⇒ public). `now_unix` in seconds.
    pub fn effective_tier(&self, now_unix: i64) -> &str {
        match self.expires_at {
            Some(exp) if now_unix > exp => "public",
            _ => &self.tier,
        }
    }
}

// NOTE: Studio intentionally ships NO tier/role *gates* here. Per this module's
// trust boundary, auth establishes who the operator is; what they may *approve*
// stays with the core's `SignerRoster` + `Quorum`. The tier/role are read for
// display + the Account-Hub upgrade path only. The web RPs (which DO gate routes)
// carry the `requireTier`/`requireRole` primitives in `@citrate/oidc-client`.

/// The hosted Account Hub URL on the issuer ("Manage account / Upgrade") — the
/// single ecosystem-wide entry point to complete or upgrade KYC. Mirrors the TS
/// `accountHubUrl`.
pub fn account_hub_url(cfg: &AuthConfig, return_to: Option<&str>) -> String {
    let base = format!("{}/account", cfg.issuer.trim_end_matches('/'));
    match return_to {
        Some(r) => format!("{base}?return_to={}", pct(r)),
        None => base,
    }
}

/// Claims we care about, extracted from the ID token. In `citrate-identity`,
/// `sub` == `wallet_address` == an EIP-55 address.
#[derive(Debug, Clone, Default)]
pub struct Claims {
    pub wallet_address: String, // == the `sub` claim (EIP-55 address)
    pub kyc_status: Option<String>,
    pub entitlement: Option<Entitlement>, // https://citrate.ai/entitlement (tier/role)
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
        entitlement: v.get(ENTITLEMENT_CLAIM).and_then(parse_entitlement),
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

// =============================================================
// CIT-STUDIO-01 / ST-B-013 — integrity seal over the stored token envelope.
//
// The ID token's RS256 signature is trusted via the TLS `/token` exchange, so it
// is not re-verified. But `session_from_store` re-reads the token from the OS
// keyring at every startup with no TLS context, so a local process that can write
// the `oidc-tokens` keyring entry could seat forged claims. We bind the stored
// bytes with an HMAC keyed by a per-install device seal key held under a SEPARATE
// keyring account: a process that overwrites only the token entry cannot forge a
// matching MAC, so tampering is detected on load (the claims still grant nothing
// in this app — this is defence in depth on the identity path).
// =============================================================

use hmac::{Hmac, Mac};
type HmacSha256 = Hmac<Sha256>;

fn seal_hex(key: &[u8; 32], token_json: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key");
    mac.update(token_json.as_bytes());
    mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

/// Wrap a token JSON string in a MAC-sealed envelope.
pub fn seal_token(key: &[u8; 32], token_json: &str) -> String {
    serde_json::json!({ "v": 1, "mac": seal_hex(key, token_json), "token": token_json }).to_string()
}

/// Open a sealed envelope, verifying the MAC in constant time. Returns the inner
/// token JSON only if the MAC matches; `None` for a tampered, unsealed, or
/// legacy-plaintext value (which forces a fresh login — fail-closed).
pub fn open_sealed_token(key: &[u8; 32], sealed: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(sealed).ok()?;
    let mac = v.get("mac")?.as_str()?;
    let token = v.get("token")?.as_str()?;
    let expect = seal_hex(key, token);
    // constant-time compare (equal length hex strings)
    if mac.len() != expect.len() {
        return None;
    }
    let differ = mac.bytes().zip(expect.bytes()).fold(0u8, |acc, (a, b)| acc | (a ^ b));
    if differ == 0 {
        Some(token.to_string())
    } else {
        None
    }
}

const SEAL_KEY_SERVICE: &str = "citrate-studio";
const SEAL_KEY_ACCOUNT: &str = "oidc-seal-key";

/// The per-install device seal key, from a keyring account SEPARATE from the token
/// entry. Created on first use. `None` if the keyring is unavailable.
fn device_seal_key() -> Option<[u8; 32]> {
    let entry = keyring::Entry::new(SEAL_KEY_SERVICE, SEAL_KEY_ACCOUNT).ok()?;
    if let Ok(hex) = entry.get_password() {
        if hex.len() == 64 {
            let mut out = [0u8; 32];
            for (i, ch) in hex.as_bytes().chunks(2).enumerate() {
                out[i] = u8::from_str_radix(std::str::from_utf8(ch).ok()?, 16).ok()?;
            }
            return Some(out);
        }
    }
    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key).ok()?;
    let hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
    entry.set_password(&hex).ok()?;
    Some(key)
}

/// Where session tokens are persisted. Production is `KeyringTokenStore` (OS
/// keyring: macOS Keychain / Windows Credential Manager / libsecret); a
/// plaintext `FileTokenStore` exists for tests only.
pub trait TokenStore {
    fn load(&self) -> Option<TokenSet>;
    fn save(&self, tokens: &TokenSet) -> std::io::Result<()>;
    fn clear(&self) -> std::io::Result<()>;
}

/// Test-only token store — a MAC-sealed envelope on disk, so the seal/verify path
/// (CIT-STUDIO-01) is exercised without touching the OS keyring. Production uses
/// `KeyringTokenStore`.
#[cfg(test)]
pub struct FileTokenStore {
    pub path: std::path::PathBuf,
}

#[cfg(test)]
impl FileTokenStore {
    // A fixed device seal key stands in for the per-install keyring key in tests.
    const TEST_KEY: [u8; 32] = [7u8; 32];
}

#[cfg(test)]
impl TokenStore for FileTokenStore {
    fn load(&self) -> Option<TokenSet> {
        let sealed = std::fs::read_to_string(&self.path).ok()?;
        let json = open_sealed_token(&Self::TEST_KEY, &sealed)?; // None ⇒ tampered ⇒ reject
        TokenSet::from_json(&json)
    }
    fn save(&self, t: &TokenSet) -> std::io::Result<()> {
        std::fs::write(&self.path, seal_token(&Self::TEST_KEY, &t.to_json()))
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
        let stored = self.entry()?.get_password().ok()?;
        // CIT-STUDIO-01: verify the integrity seal before trusting the claims. A
        // process that overwrote only the token entry cannot forge a matching MAC,
        // so a tampered (or legacy unsealed) value is rejected → fresh login.
        let key = device_seal_key()?;
        let json = open_sealed_token(&key, &stored)?;
        TokenSet::from_json(&json)
    }
    fn save(&self, t: &TokenSet) -> std::io::Result<()> {
        let key = device_seal_key().ok_or_else(|| std::io::Error::other("seal key"))?;
        let sealed = seal_token(&key, &t.to_json());
        self.entry()
            .ok_or_else(|| std::io::Error::other("keyring entry"))?
            .set_password(&sealed)
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
    /// Effective access tier (AUTHSPINE). "public" when no entitlement claim is
    /// present — display-only here; real gating stays with the core's roster/quorum.
    pub tier: String,
    /// The operator's Citrate role from the entitlement claim, if any.
    pub role: Option<String>,
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
        // CIT-STUDIO-02: do NOT route through `cmd /C start`, which re-parses its
        // argument string (metacharacters like `&`/`"` become an injection footgun).
        // `rundll32 url.dll,FileProtocolHandler <url>` passes the URL as a single
        // argv with no shell interpretation. `validate_issuer` also rejects
        // metacharacters as defence in depth.
        let mut c = std::process::Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", url]);
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
    // ST-B-012: bound the wait. An abandoned sign-in (browser closed, user walked
    // away) must not leak a detached thread + a bound loopback port for the life of
    // the process. Default budget: 3 minutes.
    wait_for_callback_until(
        server,
        expected_state,
        std::time::Instant::now() + std::time::Duration::from_secs(180),
    )
}

/// `wait_for_callback` with an explicit deadline (testable). Returns a timeout error
/// rather than blocking forever if no callback arrives before `deadline`.
pub fn wait_for_callback_until(
    server: &tiny_http::Server,
    expected_state: &str,
    deadline: std::time::Instant,
) -> Result<String, AuthError> {
    const PAGE: &str = "<!doctype html><meta charset=utf-8><title>Citrate Studio</title>\
        <body style=\"font-family:system-ui;background:#0f2a1a;color:#cde7d6;display:grid;place-items:center;height:100vh;margin:0\">\
        <div style=\"text-align:center\"><h2 style=\"color:#8ecc09\">Signed in to Citrate Studio</h2>\
        <p>You can close this window and return to the app.</p></div>";
    loop {
        let now = std::time::Instant::now();
        if now >= deadline {
            return Err(AuthError::Io("sign-in timed out".into()));
        }
        let req = match server.recv_timeout(deadline - now) {
            Ok(Some(req)) => req,
            Ok(None) => continue, // recv timed out this slice; the loop re-checks the deadline
            Err(e) => return Err(AuthError::Io(e.to_string())),
        };
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
        return match (q.get("code"), q.get("state")) {
            (Some(code), Some(state)) if state == expected_state => Ok(code.clone()),
            (_, Some(_)) => Err(AuthError::State),
            _ => Err(AuthError::Token("no code in callback".into())),
        };
    }
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
    Ok(session_from_claims(claims, now_unix()))
}

/// Project validated claims into the display session, resolving the effective tier
/// (honoring expiry) and Citrate role from the entitlement claim.
fn session_from_claims(c: Claims, now_unix: i64) -> AuthSession {
    let tier = c.entitlement.as_ref().map(|e| e.effective_tier(now_unix)).unwrap_or("public").to_string();
    let role = c.entitlement.as_ref().and_then(|e| e.citrate_role.clone());
    AuthSession { wallet: c.wallet_address, kyc: c.kyc_status.unwrap_or_default(), tier, role }
}

/// Server-side revoke (best-effort) + local token clear. The network revoke runs only over an
/// authenticated channel — a bearer token is never POSTed to a plaintext issuer (STUDIO-20 /
/// audit F-2 tidy). Local tokens are cleared regardless, so logout always succeeds locally.
pub fn logout(cfg: &AuthConfig, store: &dyn TokenStore) -> Result<(), AuthError> {
    if cfg.validate_issuer().is_ok() {
        if let Some(t) = store.load() {
            if !t.access_token.is_empty() {
                let url = format!("{}/logout", cfg.issuer.trim_end_matches('/'));
                let _ = ureq::post(&url)
                    .set("Authorization", &format!("Bearer {}", t.access_token))
                    .call();
            }
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
    let t = store.load()?;
    if let Some(c) = decode_claims(&t.id_token) {
        if validate_claims(&c, cfg, now_unix()).is_ok() {
            return Some(session_from_claims(c, now_unix()));
        }
    }
    // expired / invalid → try one refresh
    let t2 = refresh(cfg, store).ok()?;
    let c2 = decode_claims(&t2.id_token)?;
    validate_claims(&c2, cfg, now_unix()).ok()?;
    Some(session_from_claims(c2, now_unix()))
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
    fn entitlement_claim_parsed_and_tier_resolved() {
        // AUTHSPINE S2-WP5: the entitlement claim rides the id_token; tier + role
        // are read with the same contract as the web RPs.
        let obj = serde_json::json!({
            "sub":"0xAbC","wallet_address":"0xAbC","kyc_status":"verified",
            "iss":"https://auth.citrate.ai","aud":"citrate-studio","exp":9_999_999_999i64,
            "https://citrate.ai/entitlement": {
                "tier":"confidential","orgId":"citrate","citrateRole":"auditor"
            }
        });
        let payload = URL_SAFE_NO_PAD.encode(obj.to_string().as_bytes());
        let c = decode_claims(&format!("h.{payload}.sig")).unwrap();
        let ent = c.entitlement.clone().expect("entitlement parsed");
        assert_eq!(ent.tier, "confidential");
        assert_eq!(ent.citrate_role.as_deref(), Some("auditor"));
        let s = session_from_claims(c, 1_780_000_000);
        assert_eq!(s.tier, "confidential");
        assert_eq!(s.role.as_deref(), Some("auditor"));
    }

    #[test]
    fn entitlement_absent_or_unknown_tier_is_public() {
        // No claim → public.
        let mk = |obj: serde_json::Value| {
            let p = URL_SAFE_NO_PAD.encode(obj.to_string().as_bytes());
            decode_claims(&format!("h.{p}.sig")).unwrap()
        };
        let none = mk(serde_json::json!({"sub":"0x1","iss":"https://auth.citrate.ai","aud":"citrate-studio"}));
        assert!(none.entitlement.is_none());
        assert_eq!(session_from_claims(none, 1).tier, "public");
        // Unknown tier string → fail-safe to no entitlement (public).
        let bogus = mk(serde_json::json!({
            "sub":"0x1","iss":"https://auth.citrate.ai","aud":"citrate-studio",
            "https://citrate.ai/entitlement": {"tier":"superadmin"}
        }));
        assert!(bogus.entitlement.is_none());
    }

    #[test]
    fn tier_rank_ladder_and_expiry_collapse() {
        // ladder matches the TS TIER_ORDER.
        assert!(tier_rank("commercial.kyc") > tier_rank("commercial"));
        assert!(tier_rank("confidential") > tier_rank("academic"));
        assert_eq!(tier_rank("public"), 0);
        assert_eq!(tier_rank("bogus"), 0);
        // an expired entitlement collapses to public at the session boundary.
        let expired = Entitlement { tier: "confidential".into(), expires_at: Some(50), ..Default::default() };
        assert_eq!(expired.effective_tier(100), "public");
        assert_eq!(expired.effective_tier(10), "confidential");
        let live = Entitlement { tier: "academic".into(), ..Default::default() };
        assert_eq!(live.effective_tier(9_999), "academic");
    }

    #[test]
    fn account_hub_url_builds_with_return_to() {
        let cfg = AuthConfig::default(); // issuer https://auth.citrate.ai
        assert_eq!(account_hub_url(&cfg, None), "https://auth.citrate.ai/account");
        assert_eq!(
            account_hub_url(&cfg, Some("https://studio.citrate.ai")),
            "https://auth.citrate.ai/account?return_to=https%3A%2F%2Fstudio.citrate.ai"
        );
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
    fn logout_skips_network_on_insecure_issuer_but_clears_local() {
        // STUDIO-20: a bad (plaintext, non-loopback) issuer must NOT receive a bearer token,
        // but local tokens are still cleared so logout always succeeds locally.
        let path = std::env::temp_dir().join(format!("citrate-studio-logout-{}.json", std::process::id()));
        let store = FileTokenStore { path };
        store
            .save(&TokenSet { id_token: "id".into(), access_token: "tok".into(), refresh_token: None, expires_at: 0 })
            .unwrap();
        let cfg = AuthConfig { issuer: "http://evil.example".into(), ..AuthConfig::default() };
        assert!(logout(&cfg, &store).is_ok());
        assert!(store.load().is_none(), "local tokens cleared regardless of issuer");
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

    // CIT-STUDIO-02: an issuer carrying a shell/URL metacharacter (an injection
    // footgun for a launcher that re-parses its argument) must be rejected before
    // it can reach `open_url`.
    #[test]
    fn validate_issuer_rejects_shell_metacharacters() {
        for bad in [
            "https://auth.citrate.ai/ & calc.exe",
            "https://auth.citrate.ai\"quote",
            "https://auth.citrate.ai|pipe",
            "https://auth.citrate.ai;rm",
            "https://auth.citrate.ai<redir",
            "https://auth citrate.ai", // whitespace
        ] {
            assert!(
                AuthConfig { issuer: bad.into(), ..AuthConfig::default() }.validate_issuer().is_err(),
                "issuer must be rejected: {bad:?}"
            );
        }
        // the clean production issuer still validates
        assert!(AuthConfig { issuer: "https://auth.citrate.ai".into(), ..AuthConfig::default() }.validate_issuer().is_ok());
    }

    // ST-B-012: an abandoned sign-in must not block forever. With a deadline and no
    // callback, `wait_for_callback_until` returns a timeout error, so the caller can
    // drop the listener (freeing the thread + loopback port).
    #[test]
    fn loopback_wait_times_out_when_no_callback_arrives() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(200);
        let start = std::time::Instant::now();
        let r = wait_for_callback_until(&server, "state-xyz", deadline);
        assert!(matches!(r, Err(AuthError::Io(ref s)) if s.contains("timed out")), "got {r:?}");
        assert!(start.elapsed() < std::time::Duration::from_secs(5), "returned promptly");
    }

    // CIT-STUDIO-01: the token seal round-trips and detects a tampered payload.
    #[test]
    fn token_seal_roundtrips_and_detects_tamper() {
        let key = [9u8; 32];
        let json = r#"{"id_token":"h.p.s","access_token":"a","refresh_token":null,"expires_at":0}"#;
        let sealed = seal_token(&key, json);
        assert_eq!(open_sealed_token(&key, &sealed).as_deref(), Some(json), "valid seal opens");
        // a different key does not open it
        assert!(open_sealed_token(&[0u8; 32], &sealed).is_none());
        // a tampered token field is rejected (MAC no longer matches)
        let tampered = sealed.replace("h.p.s", "h.EVIL.s");
        assert!(open_sealed_token(&key, &tampered).is_none(), "tamper detected");
        // a legacy unsealed value is rejected (fail-closed → fresh login)
        assert!(open_sealed_token(&key, json).is_none());
    }

    // CIT-STUDIO-01 (the tripwire): session_from_store must reject a stored token
    // whose sealed payload was altered by another local process.
    #[test]
    fn session_from_store_rejects_a_tampered_stored_token() {
        let path = std::env::temp_dir().join(format!("citrate-studio-seal-{}.json", std::process::id()));
        let store = FileTokenStore { path: path.clone() };
        let payload = URL_SAFE_NO_PAD.encode(
            br#"{"sub":"0xAbC","wallet_address":"0xAbC","iss":"https://auth.citrate.ai","aud":"citrate-studio","exp":9999999999}"#,
        );
        let jwt = format!("eyJhbGciOiJSUzI1NiJ9.{payload}.sig");
        store
            .save(&TokenSet { id_token: jwt, access_token: "a".into(), refresh_token: None, expires_at: 0 })
            .unwrap();
        let cfg = AuthConfig::default();
        // A well-sealed token yields a session.
        assert!(session_from_store(&cfg, &store).is_some(), "intact sealed token restores a session");

        // Now tamper the stored envelope's token bytes directly on disk (simulating a
        // process that can write only the token entry) WITHOUT recomputing the MAC.
        let sealed = std::fs::read_to_string(&path).unwrap();
        let mut env: serde_json::Value = serde_json::from_str(&sealed).unwrap();
        let evil_payload = URL_SAFE_NO_PAD.encode(
            br#"{"sub":"0xEVIL","wallet_address":"0xEVIL","iss":"https://auth.citrate.ai","aud":"citrate-studio","exp":9999999999,"citrateRole":"auditor"}"#,
        );
        let evil_jwt = format!("eyJhbGciOiJSUzI1NiJ9.{evil_payload}.sig");
        let forged = TokenSet { id_token: evil_jwt, access_token: "a".into(), refresh_token: None, expires_at: 0 }.to_json();
        env["token"] = serde_json::Value::String(forged); // stale MAC now covers the wrong bytes
        std::fs::write(&path, env.to_string()).unwrap();

        assert!(
            session_from_store(&cfg, &store).is_none(),
            "a tampered stored token must be rejected"
        );
        let _ = std::fs::remove_file(&path);
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
