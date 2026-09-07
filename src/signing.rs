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

// `dev-filebacked` seeds a signer roster and writes its PRIVATE KEYS to disk in
// plaintext (`to_json_with_secrets` in `load_or_seed` below). Cargo.toml has said
// "NEVER enable in a shipping build (audit F-3)" since that audit — but nothing
// enforced it, and `cargo check --release --features dev-filebacked` succeeded
// (verified 2026-08-01). A prohibition carried only by a comment is a convention,
// not a control, and this one guards private signing material in a T1 binary.
//
// The release workflow builds `cargo build --release --locked` with no features, so
// nothing shipped has ever carried it. This makes that a fact the compiler enforces
// rather than a habit the build command happens to keep.
#[cfg(all(feature = "dev-filebacked", not(debug_assertions)))]
compile_error!(
    "`dev-filebacked` seeds a signer roster and persists its PRIVATE KEYS to disk in \
     plaintext. It is a demo/screenshot affordance and must never be compiled into an \
     optimized build (audit F-3). Drop `--features dev-filebacked`, or build in debug \
     if you are producing screenshots."
);

// ST-B-011: the guard above rides `debug_assertions`, which a profile override
// (`profile.release.debug-assertions = true`) can flip back on inside an optimized
// build, bypassing it. `shipping_profile` is set by build.rs whenever OPT_LEVEL != 0,
// i.e. gated on the optimization level itself — the property that actually matters —
// so a dev-filebacked build with optimizations on ALSO fails to compile.
#[cfg(all(feature = "dev-filebacked", shipping_profile))]
compile_error!(
    "`dev-filebacked` must never be compiled into an OPTIMIZED build (OPT_LEVEL != 0). \
     This fires even when `profile.release.debug-assertions = true` masks the \
     debug_assertions guard (audit ST-B-011). Build with opt-level 0 for demo screenshots."
);

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use zeroize::Zeroizing;

fn random_secret() -> Zeroizing<[u8; 32]> {
    let mut b = Zeroizing::new([0u8; 32]);
    getrandom::getrandom(b.as_mut()).expect("OS entropy");
    b
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Hex-encode secret bytes into a string that wipes itself on drop, so the
/// plaintext key never lingers in a freed `String`'s heap (audit ST-B-008).
fn to_hex_secret(bytes: &[u8; 32]) -> Zeroizing<String> {
    Zeroizing::new(to_hex(bytes))
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

/// Generate a fresh ed25519 keypair: returns (secret, pubkey). The secret is a
/// `Zeroizing<[u8;32]>` so it wipes itself on drop (audit ST-B-008).
pub fn gen_keypair() -> (Zeroizing<[u8; 32]>, [u8; 32]) {
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
///
/// The secret is a `Zeroizing<[u8;32]>` so every copy wipes itself on drop, and
/// `Debug` is hand-written to redact it — `#[derive(Debug)]` would otherwise
/// print the raw private-key bytes (audit ST-B-008).
#[derive(Clone)]
pub struct EnrolledSigner {
    pub role: String,
    pub name: String,
    pub pubkey: [u8; 32],
    pub secret: Option<Zeroizing<[u8; 32]>>, // FileBacked only
    pub signer_id: String,
    pub surface: String, // FileBacked | Keyring | PIV | FIDO2
    pub wallet: String,  // the authenticated wallet that enrolled (optional)
}

impl std::fmt::Debug for EnrolledSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnrolledSigner")
            .field("role", &self.role)
            .field("name", &self.name)
            .field("pubkey", &to_hex(&self.pubkey))
            .field("secret", &self.secret.as_ref().map(|_| "<redacted>"))
            .field("signer_id", &self.signer_id)
            .field("surface", &self.surface)
            .field("wallet", &self.wallet)
            .finish()
    }
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

/// Best-effort restrict a just-written file to owner read/write only (0o600).
/// No-op on non-unix. Keeps signer material out of a world-readable file
/// (audit ST-B-011 / ST-B-020).
fn set_owner_only(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

/// Load a Keyring-surface secret from the OS keyring. Used by `sign_for` (the
/// default-build / test signing path); the `core-live` binary signs through the
/// runtime's `Ed25519FileSurface` instead, so this is unused there.
fn keyring_secret(signer_id: &str) -> Option<Zeroizing<[u8; 32]>> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, signer_id).ok()?;
    let hex = Zeroizing::new(entry.get_password().ok()?); // wiped on drop
    from_hex32(&hex).map(Zeroizing::new)
}
/// Store a Keyring-surface secret in the OS keyring.
fn keyring_store(signer_id: &str, secret: &[u8; 32]) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, signer_id) {
        let hex = to_hex_secret(secret); // wiped on drop
        let _ = entry.set_password(&hex);
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
        // ST-B-020: the security decision ("do we retain a plaintext key?") must not
        // ride a permissive wildcard. Only the two explicit literals are honoured:
        // "Keyring" stores to the OS keyring; "FileBacked" keeps the secret in memory
        // (dev). ANY other value — a typo like "keyring", the stubbed "PIV"/"FIDO2"
        // surfaces, or "" — keeps NO plaintext secret, so a mistyped surface can never
        // accidentally strand a private key in process memory (it fails closed at
        // `sign_for` with `SurfaceNotWired`, exactly as PIV/FIDO2 already do).
        let kept_secret = match surface {
            "Keyring" => {
                keyring_store(&id, &secret);
                None
            }
            "FileBacked" => Some(secret), // dev only, explicit opt-in
            _ => None,                    // unknown/typo/PIV/FIDO2 → never retain plaintext
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

    /// True iff a signer *row* exists for `role`. This is NOT "can approve" —
    /// a PIV/FIDO2 stub row and a production-loaded FileBacked row (whose secret
    /// was correctly dropped on serialization) both have a row but no usable key.
    /// Renamed from the misleading `authorized` (audit ST-B-020): callers deciding
    /// whether a role can actually sign must use `can_sign`.
    pub fn has_enrolled_row(&self, role: &str) -> bool {
        self.signers.iter().any(|s| s.role == role)
    }

    /// True iff `role` has an enrolled signer with a key this build can sign with —
    /// a Keyring signer (secret in the OS keyring) or a FileBacked signer whose
    /// secret is present in memory. The real fail-closed authorization check.
    pub fn can_sign(&self, role: &str) -> bool {
        self.signers.iter().any(|s| {
            s.role == role
                && match s.surface.as_str() {
                    "Keyring" => keyring_secret(&s.signer_id).is_some(),
                    "FileBacked" => s.secret.is_some(),
                    _ => false,
                }
        })
    }

    /// Sign `payload` as `role` with the enrolled key; verifies before returning.
    /// Returns the signature + the key-storage surface. This is the default-build
    /// dock signing path; under `core-live` the dock signs through the runtime's
    /// `ApprovalQueue` (`Ed25519FileSurface`), so the `core-live` binary doesn't
    /// call this (tests still do).
    #[cfg_attr(feature = "core-live", allow(dead_code))]
    pub fn sign_for(&self, role: &str, payload: &[u8]) -> Result<([u8; 64], String), SignError> {
        let s = self.signer_for(role).ok_or_else(|| SignError::NoSigner(role.into()))?;
        let secret: Zeroizing<[u8; 32]> = match s.surface.as_str() {
            "FileBacked" => s.secret.clone().ok_or(SignError::NoKey)?,
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
    /// Production serializer — pubkey / role / signer_id / surface / wallet only, NEVER a
    /// private key (audit F-3: no plaintext secret on disk in a shipping build). Keyring
    /// secrets live in the OS keyring; a FileBacked secret is simply not persisted in
    /// production (the Settings banner warns when one is in use).
    pub fn to_json(&self) -> String {
        self.to_json_impl(false)
    }

    /// Dev/test serializer that also persists FileBacked secrets, so the demo seed survives a
    /// restart and round-trip tests can sign. Compiled only for tests / the `dev-filebacked`
    /// demo build — never reachable from a shipping binary.
    #[cfg(any(test, feature = "dev-filebacked"))]
    pub fn to_json_with_secrets(&self) -> String {
        self.to_json_impl(true)
    }

    fn to_json_impl(&self, include_secrets: bool) -> String {
        let arr: Vec<serde_json::Value> = self
            .signers
            .iter()
            .map(|s| {
                let secret = if include_secrets {
                    s.secret.as_ref().map(|sk| to_hex(sk.as_ref()))
                } else {
                    None
                };
                serde_json::json!({
                    "role": s.role,
                    "name": s.name,
                    "pubkey": to_hex(&s.pubkey),
                    "secret": secret,
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
                            secret: g("secret").and_then(|h| from_hex32(&h)).map(Zeroizing::new),
                            // ST-B-020: `signer_id` is the runtime's authorization key and the
                            // keyring lookup key. Always recompute it from the pubkey rather than
                            // trusting the value in a (0644, world-readable) roster.json — a
                            // tampered id must never be honoured verbatim.
                            signer_id: signer_id(&pubkey),
                            // ST-B-020: an ABSENT surface field must not default to the insecure
                            // "FileBacked" surface. Default to "Keyring" (secure by default);
                            // a genuine dev roster always writes its surface explicitly.
                            surface: g("surface").unwrap_or_else(|| "Keyring".into()),
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
        std::fs::write(path, self.to_json())?;
        // ST-B-011 / ST-B-020: roster.json is signer material (pubkeys, signer_ids,
        // and — on the dev surface — secrets). `std::fs::write` creates it 0644
        // (world-readable); tighten it to owner-only so another local user cannot
        // read it. Best-effort, unix only.
        set_owner_only(path);
        Ok(())
    }
    pub fn load_from(path: &std::path::Path) -> Option<Roster> {
        Some(Roster::from_json(&std::fs::read_to_string(path).ok()?))
    }

    /// Seed the five demo signers with real generated FileBacked keypairs, so the approval
    /// flow works with real crypto out of the box. Dev/test only (audit F-3): a shipping build
    /// seeds nothing — see `load_or_seed`.
    #[cfg(any(test, feature = "dev-filebacked"))]
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

/// Load the persisted roster. On first run a shipping build seeds **nothing** — High/Critical
/// actions fail closed until the operator enrolls signers (onto Keyring) deliberately
/// (audit F-3 / planset Q1). The `dev-filebacked` demo build seeds + persists a FileBacked
/// roster so screenshots/demos work without keychain prompts.
pub fn load_or_seed() -> Roster {
    if let Some(p) = roster_path() {
        if let Some(r) = Roster::load_from(&p) {
            if !r.signers.is_empty() {
                return r;
            }
        }
        #[cfg(feature = "dev-filebacked")]
        {
            let r = Roster::seed_demo();
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&p, r.to_json_with_secrets());
            set_owner_only(&p); // dev seed writes plaintext keys — owner-only (ST-B-011)
            return r;
        }
        #[cfg(not(feature = "dev-filebacked"))]
        return Roster::default(); // fail-closed: no auto-seeded keys in a shipping build.
    }
    #[cfg(feature = "dev-filebacked")]
    {
        Roster::seed_demo()
    }
    #[cfg(not(feature = "dev-filebacked"))]
    {
        Roster::default()
    }
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
        assert!(!r.has_enrolled_row("Reviewer"));
        assert!(matches!(r.sign_for("Reviewer", b"x"), Err(SignError::NoSigner(_))));

        r.enroll("Reviewer", "Dorian Vale", "FileBacked", "0xabc");
        assert!(r.has_enrolled_row("Reviewer"));
        let (sig, surface) = r.sign_for("Reviewer", b"payload").unwrap();
        assert_eq!(surface, "FileBacked");
        let pk = r.signer_for("Reviewer").unwrap().pubkey;
        assert!(verify(&pk, b"payload", &sig), "dock signature is a real, verifiable ed25519 sig");

        r.disenroll("Reviewer");
        assert!(!r.has_enrolled_row("Reviewer"));
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
        // The dev/test serializer keeps secrets so a FileBacked signer survives the round-trip.
        let json = r.to_json_with_secrets();
        let back = Roster::from_json(&json);
        assert_eq!(back.signers.len(), 5);
        // a seeded FileBacked signer can still sign after a round-trip
        let (sig, _) = back.sign_for("ComplianceOfficer", b"p").unwrap();
        let pk = back.signer_for("ComplianceOfficer").unwrap().pubkey;
        assert!(verify(&pk, b"p", &sig));
        // disenroll persists
        r.disenroll("Auditor");
        assert_eq!(Roster::from_json(&r.to_json_with_secrets()).signers.len(), 4);
    }

    #[test]
    fn production_to_json_never_writes_a_secret() {
        // audit F-3: the production serializer (used by save_to / persist_roster) must never
        // put a private key on disk, even for FileBacked signers.
        let r = Roster::seed_demo(); // 5 FileBacked signers, each with a secret in memory
        let json = r.to_json();
        assert!(!json.contains("\"secret\":\""), "no private key value in production JSON");
        let back = Roster::from_json(&json);
        assert_eq!(back.signers.len(), 5, "pubkey/role/id still persist");
        assert!(back.signers.iter().all(|s| s.secret.is_none()), "no secret survives production serialization");
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

    // ST-B-008: an enrolled signer's `Debug` must NOT print the private-key bytes.
    // A `#[derive(Debug)]` on a `[u8;32]` secret would spill it into any log line.
    #[test]
    fn debug_of_a_signer_redacts_the_secret() {
        let mut r = Roster::default();
        r.enroll("Reviewer", "Dorian", "FileBacked", "0xabc");
        let s = r.signer_for("Reviewer").unwrap();
        let raw = s.secret.as_ref().unwrap();
        let hex: String = raw.iter().map(|b| format!("{b:02x}")).collect();
        let dbg = format!("{s:?}");
        assert!(dbg.contains("<redacted>"), "secret must render redacted: {dbg}");
        assert!(!dbg.contains(&hex), "the private-key hex must never appear in Debug output");
        // and the whole roster's Debug (derived) inherits the redaction
        assert!(!format!("{r:?}").contains(&hex));
    }

    // ST-B-020: the security decision "retain a plaintext key?" must not ride a
    // permissive wildcard — only the explicit "FileBacked" literal keeps a secret;
    // a typo, "", or a stub surface keeps NONE.
    #[test]
    fn enroll_never_retains_a_plaintext_key_for_an_unknown_surface() {
        for surface in ["keyring", "KEYRING", "PIV", "FIDO2", "", "filebacked"] {
            let mut r = Roster::default();
            r.enroll("Reviewer", "Dorian", surface, "0xabc");
            let s = r.signer_for("Reviewer").unwrap();
            assert!(
                s.secret.is_none(),
                "surface {surface:?} must not strand a plaintext key in memory"
            );
            assert!(!r.can_sign("Reviewer"), "an unusable surface can't sign");
        }
        // the two explicit, honoured literals still behave as designed
        let mut r = Roster::default();
        r.enroll("Reviewer", "Dorian", "FileBacked", "0xabc");
        assert!(r.signer_for("Reviewer").unwrap().secret.is_some());
        assert!(r.can_sign("Reviewer"));
    }

    // ST-B-020: `has_enrolled_row` is presence, `can_sign` is authority. A stub
    // (keyless) row is enrolled but cannot approve.
    #[test]
    fn has_row_is_not_can_sign_for_a_keyless_surface() {
        let mut r = Roster::default();
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
        assert!(r.has_enrolled_row("SecurityOfficer"), "the row exists");
        assert!(!r.can_sign("SecurityOfficer"), "but it has no usable key");
    }

    // ST-B-020: a roster.json whose `signer_id` was tampered must be re-derived from
    // the pubkey on load, never trusted verbatim; and an absent `surface` must
    // default to the secure "Keyring", not the insecure "FileBacked".
    #[test]
    fn from_json_recomputes_signer_id_and_defaults_surface_securely() {
        let (_, pubkey) = gen_keypair();
        let real = signer_id(&pubkey);
        let json = serde_json::json!({
            "signers": [{
                "role": "Reviewer",
                "name": "Dorian",
                "pubkey": to_hex(&pubkey),
                "signer_id": "deadbeef".repeat(8), // an attacker-supplied id
                // note: no "surface" field
            }]
        })
        .to_string();
        let r = Roster::from_json(&json);
        let s = r.signer_for("Reviewer").unwrap();
        assert_eq!(s.signer_id, real, "signer_id is recomputed, not trusted");
        assert_ne!(s.signer_id, "deadbeef".repeat(8));
        assert_eq!(s.surface, "Keyring", "absent surface defaults to the secure one");
    }

    // ST-B-011 / ST-B-020: a persisted roster.json must be owner-only (0o600), not
    // the 0644 world-readable file `std::fs::write` creates by default.
    #[cfg(unix)]
    #[test]
    fn saved_roster_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!("citrate-roster-perm-{}.json", std::process::id()));
        Roster::seed_demo().save_to(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "roster.json must be 0o600, got {mode:o}");
        let _ = std::fs::remove_file(&path);
    }
}
