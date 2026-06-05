//! Citrate Studio — signer roster + real ed25519 key enrollment (STUDIO-4).
//!
//! Turns an authenticated identity into a role-bearing signer with a *real*
//! ed25519 key. The approval dock signs the gate payload with the enrolled
//! key and verifies it before it counts — replacing the demo roster's fake
//! fingerprints. `signer_id = SHA-256(pubkey)` is byte-identical to the
//! runtime's `hitl::signing::signer_id_from_pubkey`, so an enrolled signer's id
//! matches what the core's `SignerRoster` records.
//!
//! Signing surfaces are tiered: FileBacked (dev — secret on disk) and Keyring
//! (secret in the OS keyring) work today; PIV/CAC and FIDO2 (attested hardware)
//! are stubbed behind a clear seam for the live `SigningSurfaceTag::Slint` path.

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn random_secret() -> [u8; 32] {
    let mut b = [0u8; 32];
    getrandom::getrandom(&mut b).expect("OS entropy");
    b
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        out[i] = u8::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
    }
    Some(out)
}

/// `signer_id = SHA-256(pubkey)` as 64-char hex — matches the runtime.
pub fn signer_id(pubkey: &[u8; 32]) -> String {
    to_hex(&Sha256::digest(pubkey))
}

/// Generate a fresh ed25519 keypair: returns (secret, pubkey).
pub fn gen_keypair() -> ([u8; 32], [u8; 32]) {
    let secret = random_secret();
    let sk = SigningKey::from_bytes(&secret);
    (secret, sk.verifying_key().to_bytes())
}

/// Sign `msg` with an ed25519 secret key.
pub fn sign(secret: &[u8; 32], msg: &[u8]) -> [u8; 64] {
    SigningKey::from_bytes(secret).sign(msg).to_bytes()
}

/// Verify an ed25519 signature against a public key.
pub fn verify(pubkey: &[u8; 32], msg: &[u8], sig: &[u8; 64]) -> bool {
    match VerifyingKey::from_bytes(pubkey) {
        Ok(vk) => vk.verify(msg, &ed25519_dalek::Signature::from_bytes(sig)).is_ok(),
        Err(_) => false,
    }
}

/// Errors from the signing surfaces.
#[derive(Debug, PartialEq)]
pub enum SignError {
    NoSigner(String),       // no enrolled signer for this role
    NoKey,                  // surface has no usable key
    SurfaceNotWired(String),// PIV/FIDO2 — seam present, not implemented
    VerifyFailed,           // produced signature didn't verify (should never happen)
}
impl std::fmt::Display for SignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignError::NoSigner(r) => write!(f, "no enrolled signer for role {r}"),
            SignError::NoKey => write!(f, "no usable key on this signing surface"),
            SignError::SurfaceNotWired(s) => write!(f, "{s} signing surface is not yet wired"),
            SignError::VerifyFailed => write!(f, "signature failed self-verification"),
        }
    }
}

/// One enrolled signer. `secret` is present only for the FileBacked (dev)
/// surface; Keyring secrets live in the OS keyring (keyed by `signer_id`).
#[derive(Debug, Clone)]
pub struct EnrolledSigner {
    pub role: String,
    pub name: String,
    pub pubkey: [u8; 32],
    pub secret: Option<[u8; 32]>, // FileBacked only
    pub signer_id: String,
    pub surface: String, // FileBacked | Keyring | PIV | FIDO2
    pub wallet: String,  // the authenticated wallet that enrolled (optional)
}

impl EnrolledSigner {
    /// Display fingerprint: first 4 + last 4 of the signer_id.
    pub fn fp(&self) -> String {
        if self.signer_id.len() >= 8 {
            format!("{}…{}", &self.signer_id[..4], &self.signer_id[self.signer_id.len() - 4..])
        } else {
            self.signer_id.clone()
        }
    }
}

const KEYRING_SERVICE: &str = "citrate-studio-signer";

/// Load a Keyring-surface secret from the OS keyring. Used by `sign_for` (the
/// default-build / test signing path); the `core-live` binary signs through the
/// runtime's `Ed25519FileSurface` instead, so this is unused there.
#[cfg_attr(feature = "core-live", allow(dead_code))]
fn keyring_secret(signer_id: &str) -> Option<[u8; 32]> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, signer_id).ok()?;
    from_hex32(&entry.get_password().ok()?)
}
/// Store a Keyring-surface secret in the OS keyring.
fn keyring_store(signer_id: &str, secret: &[u8; 32]) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, signer_id) {
        let _ = entry.set_password(&to_hex(secret));
    }
}
fn keyring_clear(signer_id: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, signer_id) {
        let _ = entry.delete_password();
    }
}

/// The roster of enrolled signers. Persisted as JSON; private keys for Keyring
/// signers live in the OS keyring, not the file.
#[derive(Debug, Clone, Default)]
pub struct Roster {
    pub signers: Vec<EnrolledSigner>,
}

impl Roster {
    /// Enroll a fresh signer for `role` on `surface`. Generates a keypair; the
    /// secret goes to the keyring for Keyring surfaces, into the struct (and the
    /// dev file) for FileBacked. Replaces any existing signer for that role.
    pub fn enroll(&mut self, role: &str, name: &str, surface: &str, wallet: &str) -> &EnrolledSigner {
        let (secret, pubkey) = gen_keypair();
        let id = signer_id(&pubkey);
        let kept_secret = match surface {
            "Keyring" => {
                keyring_store(&id, &secret);
                None
            }
            _ => Some(secret), // FileBacked (dev)
        };
        self.signers.retain(|s| s.role != role);
        self.signers.push(EnrolledSigner {
            role: role.into(),
            name: name.into(),
            pubkey,
            secret: kept_secret,
            signer_id: id,
            surface: surface.into(),
            wallet: wallet.into(),
        });
        self.signers.last().unwrap()
    }

    /// Remove the signer for `role` (also clears its keyring secret).
    pub fn disenroll(&mut self, role: &str) {
        if let Some(s) = self.signers.iter().find(|s| s.role == role) {
            if s.surface == "Keyring" {
                keyring_clear(&s.signer_id);
            }
        }
        self.signers.retain(|s| s.role != role);
    }

    pub fn signer_for(&self, role: &str) -> Option<&EnrolledSigner> {
        self.signers.iter().find(|s| s.role == role)
    }

    /// True iff some signer is enrolled for `role` (the fail-closed check).
    pub fn authorized(&self, role: &str) -> bool {
        self.signers.iter().any(|s| s.role == role)
    }

    /// Sign `payload` as `role` with the enrolled key; verifies before returning.
    /// Returns the signature + the key-storage surface. This is the default-build
    /// dock signing path; under `core-live` the dock signs through the runtime's
    /// `ApprovalQueue` (`Ed25519FileSurface`), so the `core-live` binary doesn't
    /// call this (tests still do).
    #[cfg_attr(feature = "core-live", allow(dead_code))]
    pub fn sign_for(&self, role: &str, payload: &[u8]) -> Result<([u8; 64], String), SignError> {
        let s = self.signer_for(role).ok_or_else(|| SignError::NoSigner(role.into()))?;
        let secret = match s.surface.as_str() {
            "FileBacked" => s.secret.ok_or(SignError::NoKey)?,
            "Keyring" => keyring_secret(&s.signer_id).ok_or(SignError::NoKey)?,
            other => return Err(SignError::SurfaceNotWired(other.into())),
        };
        let sig = sign(&secret, payload);
        if !verify(&s.pubkey, payload, &sig) {
            return Err(SignError::VerifyFailed);
        }
        Ok((sig, s.surface.clone()))
    }

    // ---- persistence ----
    pub fn to_json(&self) -> String {
        let arr: Vec<serde_json::Value> = self
            .signers
            .iter()
            .map(|s| {
                serde_json::json!({
                    "role": s.role,
                    "name": s.name,
                    "pubkey": to_hex(&s.pubkey),
                    "secret": s.secret.map(|sk| to_hex(&sk)),
                    "signer_id": s.signer_id,
                    "surface": s.surface,
                    "wallet": s.wallet,
                })
            })
            .collect();
        serde_json::json!({ "signers": arr }).to_string()
    }

    pub fn from_json(s: &str) -> Roster {
        let v: serde_json::Value = match serde_json::from_str(s) {
            Ok(v) => v,
            Err(_) => return Roster::default(),
        };
        let signers = v
            .get("signers")
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| {
                        let g = |k: &str| e.get(k).and_then(|x| x.as_str()).map(|x| x.to_string());
                        let pubkey = from_hex32(&g("pubkey")?)?;
                        Some(EnrolledSigner {
                            role: g("role")?,
                            name: g("name").unwrap_or_default(),
                            pubkey,
                            secret: g("secret").and_then(|h| from_hex32(&h)),
                            signer_id: g("signer_id").unwrap_or_else(|| signer_id(&pubkey)),
                            surface: g("surface").unwrap_or_else(|| "FileBacked".into()),
                            wallet: g("wallet").unwrap_or_default(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Roster { signers }
    }

    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.to_json())
    }
    pub fn load_from(path: &std::path::Path) -> Option<Roster> {
        Some(Roster::from_json(&std::fs::read_to_string(path).ok()?))
    }

    /// Seed the five demo signers with real generated FileBacked keypairs, so
    /// the approval flow works with real crypto out of the box.
    pub fn seed_demo() -> Roster {
        let mut r = Roster::default();
        for (role, name) in [
            ("Operator", "Aleia Rouhani"),
            ("Reviewer", "Dorian Vale"),
            ("ComplianceOfficer", "Priya Anand"),
            ("SecurityOfficer", "Marcus Greel"),
            ("Auditor", "Ext. Auditor (read-only)"),
        ] {
            r.enroll(role, name, "FileBacked", "did:citrate:op:aleia");
        }
        r
    }
}

/// The platform config directory for citrate-studio (dep-free).
pub fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support/citrate-studio"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("citrate-studio"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|c| c.join("citrate-studio"))
    }
}

pub fn roster_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("roster.json"))
}

/// Load the persisted roster, or seed + persist the demo roster on first run.
pub fn load_or_seed() -> Roster {
    if let Some(p) = roster_path() {
        if let Some(r) = Roster::load_from(&p) {
            if !r.signers.is_empty() {
                return r;
            }
        }
        let r = Roster::seed_demo();
        let _ = r.save_to(&p);
        return r;
    }
    Roster::seed_demo()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keypair_sign_verify_roundtrip() {
        let (secret, pubkey) = gen_keypair();
        let msg = b"sha256:7b41...e0c9";
        let sig = sign(&secret, msg);
        assert!(verify(&pubkey, msg, &sig), "valid signature verifies");
        // wrong message fails
        assert!(!verify(&pubkey, b"tampered", &sig));
        // wrong key fails
        let (_, other) = gen_keypair();
        assert!(!verify(&other, msg, &sig));
    }

    #[test]
    fn signer_id_matches_runtime_scheme() {
        // SHA-256 of 32 zero bytes (same vector as the auth scaffold + runtime).
        assert_eq!(&signer_id(&[0u8; 32])[..8], "66687aad");
        assert_eq!(signer_id(&[0u8; 32]).len(), 64);
    }

    #[test]
    fn enroll_sign_and_fail_closed() {
        let mut r = Roster::default();
        // Fail-closed: nobody enrolled.
        assert!(!r.authorized("Reviewer"));
        assert!(matches!(r.sign_for("Reviewer", b"x"), Err(SignError::NoSigner(_))));

        r.enroll("Reviewer", "Dorian Vale", "FileBacked", "0xabc");
        assert!(r.authorized("Reviewer"));
        let (sig, surface) = r.sign_for("Reviewer", b"payload").unwrap();
        assert_eq!(surface, "FileBacked");
        let pk = r.signer_for("Reviewer").unwrap().pubkey;
        assert!(verify(&pk, b"payload", &sig), "dock signature is a real, verifiable ed25519 sig");

        r.disenroll("Reviewer");
        assert!(!r.authorized("Reviewer"));
    }

    #[test]
    fn piv_surface_is_stubbed_not_faked() {
        let mut r = Roster::default();
        // manually craft a PIV signer (no real key — the hardware seam)
        let (_, pubkey) = gen_keypair();
        r.signers.push(EnrolledSigner {
            role: "SecurityOfficer".into(),
            name: "Marcus".into(),
            pubkey,
            secret: None,
            signer_id: signer_id(&pubkey),
            surface: "PIV".into(),
            wallet: "".into(),
        });
        assert!(matches!(r.sign_for("SecurityOfficer", b"x"), Err(SignError::SurfaceNotWired(_))));
    }

    #[test]
    fn roster_persist_roundtrip() {
        let mut r = Roster::seed_demo();
        assert_eq!(r.signers.len(), 5);
        let json = r.to_json();
        let back = Roster::from_json(&json);
        assert_eq!(back.signers.len(), 5);
        // a seeded FileBacked signer can still sign after a round-trip
        let (sig, _) = back.sign_for("ComplianceOfficer", b"p").unwrap();
        let pk = back.signer_for("ComplianceOfficer").unwrap().pubkey;
        assert!(verify(&pk, b"p", &sig));
        // disenroll persists
        r.disenroll("Auditor");
        assert_eq!(Roster::from_json(&r.to_json()).signers.len(), 4);
    }

    #[test]
    fn save_load_to_temp_file() {
        let path = std::env::temp_dir().join(format!("citrate-roster-test-{}.json", std::process::id()));
        let r = Roster::seed_demo();
        r.save_to(&path).unwrap();
        let back = Roster::load_from(&path).unwrap();
        assert_eq!(back.signers.len(), 5);
        let _ = std::fs::remove_file(&path);
    }
}
