//! Citrate Studio — persisted setup config (STUDIO-5).
//!
//! Onboarding writes a `citrate-studio.toml` in the platform config dir. Its
//! presence (with `onboarding_complete = true`) is what routes a returning user
//! straight to Studio instead of replaying setup. The file is a flat, human-
//! readable TOML subset (string/bool/int key = value) — written and read here,
//! no serde-derive dependency.
//!
//! Also home to the two "real action" backends onboarding drives: model-runtime
//! discovery (a real HTTP probe) and capsule install (real ed25519 signature
//! verification, fail-closed on unverified).

use crate::signing;
use std::path::PathBuf;
use std::time::Duration;

/// The persisted setup configuration.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Config {
    pub workspace: String, // "Personal" | "Team"
    pub tenant: String,
    pub runtime: String,   // chosen model runtime label
    pub oversight: String, // in | on | out
    pub capsules: u32,     // count installed (signature-verified)
    pub first_prompt: String,
    pub onboarding_complete: bool,
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
fn unesc(s: &str) -> String {
    s.replace("\\\"", "\"").replace("\\\\", "\\")
}

impl Config {
    pub fn to_toml(&self) -> String {
        format!(
            "# Citrate Studio — setup configuration (written by onboarding, STUDIO-5)\n\
             workspace = \"{}\"\n\
             tenant = \"{}\"\n\
             runtime = \"{}\"\n\
             oversight = \"{}\"\n\
             capsules = {}\n\
             first_prompt = \"{}\"\n\
             onboarding_complete = {}\n",
            esc(&self.workspace),
            esc(&self.tenant),
            esc(&self.runtime),
            esc(&self.oversight),
            self.capsules,
            esc(&self.first_prompt),
            self.onboarding_complete,
        )
    }

    pub fn from_toml(s: &str) -> Config {
        let mut c = Config::default();
        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let k = k.trim();
            let v = v.trim();
            // strip exactly one delimiter quote each side (not escaped inner ones)
            let sv = || unesc(v.strip_prefix('"').and_then(|x| x.strip_suffix('"')).unwrap_or(v));
            match k {
                "workspace" => c.workspace = sv(),
                "tenant" => c.tenant = sv(),
                "runtime" => c.runtime = sv(),
                "oversight" => c.oversight = sv(),
                "first_prompt" => c.first_prompt = sv(),
                "capsules" => c.capsules = v.parse().unwrap_or(0),
                "onboarding_complete" => c.onboarding_complete = v == "true",
                _ => {}
            }
        }
        c
    }

    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.to_toml())
    }
    pub fn load_from(path: &std::path::Path) -> Option<Config> {
        Some(Config::from_toml(&std::fs::read_to_string(path).ok()?))
    }
}

/// `…/citrate-studio/citrate-studio.toml`.
pub fn config_path() -> Option<PathBuf> {
    signing::config_dir().map(|d| d.join("citrate-studio.toml"))
}

/// The persisted config, if onboarding has run.
pub fn load() -> Option<Config> {
    config_path().and_then(|p| Config::load_from(&p))
}

/// Persist the config to the platform config path (best-effort).
pub fn save(c: &Config) {
    if let Some(p) = config_path() {
        let _ = c.save_to(&p);
    }
}

/// True iff a completed setup exists — the first-launch routing signal.
pub fn is_configured() -> bool {
    load().map(|c| c.onboarding_complete).unwrap_or(false)
}

// ---- model-runtime discovery -------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeProbe {
    pub kind: String,
    pub endpoint: String,
    pub available: bool,
}

/// A real reachability probe of a local HTTP runtime (short timeout).
pub fn probe(endpoint: &str, path: &str) -> bool {
    ureq::get(&format!("{endpoint}{path}"))
        .timeout(Duration::from_millis(400))
        .call()
        .is_ok()
}

/// Discover local model runtimes. Ollama + llama.cpp are probed for real;
/// embedded Gemma is always available (bundled GGUF, verified by SHA-256).
pub fn discover_runtimes() -> Vec<RuntimeProbe> {
    vec![
        RuntimeProbe {
            kind: "Ollama".into(),
            endpoint: "http://localhost:11434".into(),
            available: probe("http://localhost:11434", "/api/tags"),
        },
        RuntimeProbe {
            kind: "llama.cpp".into(),
            endpoint: "http://localhost:8080".into(),
            available: probe("http://localhost:8080", "/health"),
        },
        RuntimeProbe {
            kind: "Embedded Gemma".into(),
            endpoint: "bundled://gemma-2b-it-Q4_K_M".into(),
            available: true,
        },
    ]
}

// ---- capsule install (fail-closed) ------------------------------------------

/// A capsule offered for install: its bytes, an ed25519 signature, and the
/// publisher's verifying key. A real `.cps` registry feed lands in STUDIO-6.
pub struct CapsuleSource {
    pub name: String,
    pub bytes: Vec<u8>,
    pub sig: [u8; 64],
    pub publisher: [u8; 32],
}

/// Pinned publisher keys — the **trust anchor** for capsule installation. A capsule is
/// accepted only if its publisher key is pinned here AND its signature verifies; a valid
/// self-signature from an unpinned key is NOT trusted (audit F-6: signature consistency is
/// integrity, not authenticity). The real `.cps` registry feed (CIT-AGENT-3e) populates this
/// from a signed source of record; today the demo pins its own generated key.
#[derive(Debug, Clone, Default)]
pub struct PublisherTrustStore {
    pinned: Vec<[u8; 32]>,
}
impl PublisherTrustStore {
    pub fn new() -> Self {
        Self { pinned: Vec::new() }
    }
    pub fn pin(&mut self, pubkey: [u8; 32]) {
        if !self.pinned.contains(&pubkey) {
            self.pinned.push(pubkey);
        }
    }
    pub fn is_pinned(&self, pubkey: &[u8; 32]) -> bool {
        self.pinned.contains(pubkey)
    }
}

/// Verify a capsule against the pinned publisher trust store: the publisher must be pinned
/// AND the signature must verify over the bytes. Authenticity (pinning) + integrity (sig).
pub fn verify_capsule(c: &CapsuleSource, trust: &PublisherTrustStore) -> bool {
    trust.is_pinned(&c.publisher) && signing::verify(&c.publisher, &c.bytes, &c.sig)
}

/// Install only capsules from a pinned publisher with a valid signature. Returns
/// (installed, rejected) — fail-closed: anything unpinned or unverified is rejected.
pub fn install_capsules(
    sources: &[CapsuleSource],
    trust: &PublisherTrustStore,
) -> (Vec<String>, Vec<String>) {
    let mut ok = Vec::new();
    let mut bad = Vec::new();
    for c in sources {
        if verify_capsule(c, trust) {
            ok.push(c.name.clone());
        } else {
            bad.push(c.name.clone());
        }
    }
    (ok, bad)
}

/// A demo capsule set signed by a single publisher key + the trust store that pins it, plus
/// one tampered capsule whose signature won't verify (to exercise fail-closed). Stands in for
/// a real signed registry feed until CIT-AGENT-3e.
pub fn demo_capsule_sources_with_trust() -> (Vec<CapsuleSource>, PublisherTrustStore) {
    let (pub_secret, publisher) = signing::gen_keypair();
    let mut trust = PublisherTrustStore::new();
    trust.pin(publisher);
    let mut sources = Vec::new();
    for name in ["recon.snapshot", "recon.pull-cui", "recon.match-phi", "recon.write-report", "recon.anchor-merkle"] {
        let bytes = format!("capsule:{name}").into_bytes();
        let sig = signing::sign(&pub_secret, &bytes);
        sources.push(CapsuleSource { name: name.into(), bytes, sig, publisher });
    }
    // one unsigned/tampered capsule — pinned publisher but invalid signature → rejected.
    sources.push(CapsuleSource {
        name: "rogue.exfiltrate".into(),
        bytes: b"capsule:rogue.exfiltrate".to_vec(),
        sig: [0u8; 64],
        publisher,
    });
    (sources, trust)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_roundtrips() {
        let c = Config {
            workspace: "Team".into(),
            tenant: "BOEING · PROCUREMENT #14".into(),
            runtime: "Ollama :11434".into(),
            oversight: "on".into(),
            capsules: 5,
            first_prompt: "Reconcile last night's \"ledger\"".into(),
            onboarding_complete: true,
        };
        let back = Config::from_toml(&c.to_toml());
        assert_eq!(c, back, "config survives a TOML round-trip incl. escaped quotes");
        assert!(back.onboarding_complete);
        assert_eq!(back.capsules, 5);
    }

    #[test]
    fn absent_config_is_unconfigured() {
        // from_toml of nothing → default, not configured
        let c = Config::from_toml("");
        assert!(!c.onboarding_complete);
        assert_eq!(c, Config::default());
    }

    #[test]
    fn save_load_temp() {
        let path = std::env::temp_dir().join(format!("citrate-studio-cfg-{}.toml", std::process::id()));
        let c = Config { onboarding_complete: true, runtime: "Embedded Gemma".into(), ..Default::default() };
        c.save_to(&path).unwrap();
        assert_eq!(Config::load_from(&path).unwrap().runtime, "Embedded Gemma");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn capsule_install_is_fail_closed() {
        let (sources, trust) = demo_capsule_sources_with_trust();
        let (ok, bad) = install_capsules(&sources, &trust);
        assert_eq!(ok.len(), 5, "the 5 signed capsules install");
        assert_eq!(bad, vec!["rogue.exfiltrate"], "the unsigned capsule is rejected");
    }

    #[test]
    fn capsule_verify_rejects_tamper() {
        let (secret, publisher) = signing::gen_keypair();
        let mut trust = PublisherTrustStore::new();
        trust.pin(publisher);
        let bytes = b"capsule:recon.snapshot".to_vec();
        let sig = signing::sign(&secret, &bytes);
        let good = CapsuleSource { name: "x".into(), bytes: bytes.clone(), sig, publisher };
        assert!(verify_capsule(&good, &trust));
        // tamper the bytes → verify fails
        let bad = CapsuleSource { name: "x".into(), bytes: b"capsule:rogue".to_vec(), sig, publisher };
        assert!(!verify_capsule(&bad, &trust));
    }

    #[test]
    fn capsule_verify_rejects_unpinned_publisher() {
        // audit F-6: a valid self-signature from an UNPINNED key is not trusted.
        let (secret, publisher) = signing::gen_keypair();
        let bytes = b"capsule:attacker".to_vec();
        let sig = signing::sign(&secret, &bytes);
        let c = CapsuleSource { name: "attacker".into(), bytes, sig, publisher };
        assert!(!verify_capsule(&c, &PublisherTrustStore::new()), "unpinned publisher rejected");
        let mut trust = PublisherTrustStore::new();
        trust.pin(publisher);
        assert!(verify_capsule(&c, &trust), "pinned publisher with a valid sig accepted");
    }

    #[test]
    fn runtime_probe_against_a_mock() {
        // a reachable server → available; an unbound port → not.
        let srv = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let port = srv.server_addr().to_ip().unwrap().port();
        let h = std::thread::spawn(move || {
            if let Some(req) = srv.incoming_requests().next() {
                let _ = req.respond(tiny_http::Response::from_string("[]"));
            }
        });
        assert!(probe(&format!("http://127.0.0.1:{port}"), "/api/tags"));
        h.join().unwrap();
        // a port nobody is listening on
        assert!(!probe("http://127.0.0.1:1", "/api/tags"));
    }
}
