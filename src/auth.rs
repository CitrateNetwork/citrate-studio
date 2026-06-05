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

/// Claims we care about, extracted from the validated ID token / userinfo.
/// In `citrate-identity`, `sub` == `wallet_address` == an EIP-55 address.
#[derive(Debug, Clone, Default)]
pub struct Claims {
    pub sub: String,
    pub wallet_address: String,
    pub kyc_status: Option<String>,
}

/// Parse the (already-validated) ID-token payload. NOTE: this only *decodes*
/// the JWT body — it does NOT verify the RS256 signature. The network commit
/// validates against `/jwks` (via `jsonwebtoken`) BEFORE calling this.
pub fn decode_claims_unverified(id_token: &str) -> Option<Claims> {
    let payload_b64 = id_token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(|x| x.to_string());
    Some(Claims {
        sub: s("sub").unwrap_or_default(),
        wallet_address: s("wallet_address").or_else(|| s("sub")).unwrap_or_default(),
        kyc_status: s("kyc_status"),
    })
}

/// Where session tokens are persisted. The real impl is the OS keyring
/// (macOS Keychain / Windows Credential Manager / libsecret) — STUDIO-2 next
/// step. `FileTokenStore` is dev-only and intentionally simple.
pub trait TokenStore {
    fn load(&self) -> Option<TokenSet>;
    fn save(&self, tokens: &TokenSet) -> std::io::Result<()>;
    fn clear(&self) -> std::io::Result<()>;
}

/// DEV ONLY token store — plaintext JSON on disk. Do not ship; the keyring
/// store replaces it. Present so the flow is testable end-to-end without a
/// keyring dependency.
pub struct FileTokenStore {
    pub path: std::path::PathBuf,
}

impl TokenStore for FileTokenStore {
    fn load(&self) -> Option<TokenSet> {
        let bytes = std::fs::read(&self.path).ok()?;
        let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        Some(TokenSet {
            id_token: v.get("id_token")?.as_str()?.to_string(),
            access_token: v.get("access_token").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            refresh_token: v.get("refresh_token").and_then(|x| x.as_str()).map(|s| s.to_string()),
            expires_at: v.get("expires_at").and_then(|x| x.as_u64()).unwrap_or(0),
        })
    }
    fn save(&self, t: &TokenSet) -> std::io::Result<()> {
        let v = serde_json::json!({
            "id_token": t.id_token,
            "access_token": t.access_token,
            "refresh_token": t.refresh_token,
            "expires_at": t.expires_at,
        });
        std::fs::write(&self.path, serde_json::to_vec_pretty(&v).unwrap_or_default())
    }
    fn clear(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

// ---- identity → roster mapping (the seam STUDIO-4 builds on) ----

/// `signer_id = SHA-256(pubkey)` as 64-char hex — identical to the runtime's
/// `signer_id_from_pubkey` (`hitl/signing.rs`), so an enrolled signer's id
/// matches what the core's `SignerRoster` records.
pub fn signer_id_from_pubkey(pubkey: &[u8; 32]) -> String {
    let digest = Sha256::digest(pubkey);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// One enrolled signer: an authenticated wallet, the ed25519 pubkey that will
/// sign approvals (file/keyring/PIV/FIDO2 — STUDIO-4), the role, and the
/// audit surface tag. Auth gives `wallet`; an admin chooses `role` + key.
#[derive(Debug, Clone)]
pub struct RosterEntry {
    pub wallet: String,      // from auth (EIP-55 address)
    pub pubkey_hex: String,  // ed25519 signer pubkey (32 bytes hex)
    pub signer_id: String,   // SHA-256(pubkey)
    pub role: String,        // Operator|Reviewer|ComplianceOfficer|SecurityOfficer|Auditor
    pub surface: String,     // SigningSurfaceTag: Slint|FileBacked|...
}

/// Build a roster entry from an authenticated wallet + a signer pubkey + role.
pub fn enroll(wallet: &str, pubkey: &[u8; 32], role: &str, surface: &str) -> RosterEntry {
    RosterEntry {
        wallet: wallet.to_string(),
        pubkey_hex: pubkey.iter().map(|b| format!("{b:02x}")).collect(),
        signer_id: signer_id_from_pubkey(pubkey),
        role: role.to_string(),
        surface: surface.to_string(),
    }
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
    fn signer_id_is_sha256_hex() {
        let id = signer_id_from_pubkey(&[0u8; 32]);
        assert_eq!(id.len(), 64);
        // sha256 of 32 zero bytes:
        assert_eq!(&id[..8], "66687aad");
    }

    #[test]
    fn decode_claims_reads_wallet() {
        // header.payload.sig — payload = {"sub":"0xAbC","wallet_address":"0xAbC"}
        let payload = URL_SAFE_NO_PAD.encode(br#"{"sub":"0xAbC","wallet_address":"0xAbC","kyc_status":"verified"}"#);
        let jwt = format!("eyJhbGciOiJSUzI1NiJ9.{payload}.sig");
        let c = decode_claims_unverified(&jwt).unwrap();
        assert_eq!(c.wallet_address, "0xAbC");
        assert_eq!(c.kyc_status.as_deref(), Some("verified"));
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
}
