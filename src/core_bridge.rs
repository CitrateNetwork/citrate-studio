//! Bridge to the real `citrate-agent-core` (compiled only under `core-live`).
//!
//! This is the seam STUDIO-3 grows through: it maps studio's UI-facing string
//! representations (role names, risk tiers) onto the core's typed enums and
//! delegates the *policy* — separation-of-duties and quorum — to the real
//! runtime functions. The default (non-`core-live`) build keeps a faithful
//! hand-rolled copy in `policy` (see main.rs); this module is the proof that
//! flipping the feature swaps in the authoritative core without any UI change.
//!
//! As later STUDIO-3 steps land, the same pattern extends to `AuditChain`
//! (`verify_integrity`), the async `ApprovalQueue`, the signed `DoctorReport`,
//! and `CapsuleDispatch`.

use citrate_agent_core::capsule::manifest::{Role, RiskTier};
use citrate_agent_core::hitl::{self, Quorum};

/// Studio renders roles as their core enum-variant names; map back to `Role`.
pub fn role_from_str(s: &str) -> Option<Role> {
    Some(match s {
        "Operator" => Role::Operator,
        "Reviewer" => Role::Reviewer,
        "ComplianceOfficer" => Role::ComplianceOfficer,
        "SecurityOfficer" => Role::SecurityOfficer,
        "Auditor" => Role::Auditor,
        _ => return None,
    })
}

/// Studio renders tiers as the manifest's kebab strings; map back to `RiskTier`.
pub fn tier_from_str(s: &str) -> Option<RiskTier> {
    Some(match s {
        "low" => RiskTier::Low,
        "medium" => RiskTier::Medium,
        "high" => RiskTier::High,
        "critical" => RiskTier::Critical,
        _ => return None,
    })
}

/// The policy seam, backed by the real core. Signatures match the default
/// `policy` module in main.rs exactly, so `#[cfg]` selection is invisible to
/// callers.
pub mod policy {
    use super::*;

    /// Separation-of-duties: delegates to the runtime's `hitl::is_conflict`
    /// (the authoritative `CO ⊥ SO` rule).
    pub fn is_conflict(a: &str, b: &str) -> bool {
        match (role_from_str(a), role_from_str(b)) {
            (Some(x), Some(y)) => hitl::is_conflict(x, y),
            _ => false,
        }
    }

    /// Whether a role may approve at all (the runtime makes Auditor read-only).
    pub fn can_approve(role: &str) -> bool {
        role_from_str(role).map(hitl::can_approve).unwrap_or(false)
    }

    /// The signature count a tier requires, from the real `Quorum::for_tier`.
    pub fn quorum_n(tier: &str, manifest_roles: &[String]) -> i32 {
        let t = match tier_from_str(tier) {
            Some(t) => t,
            None => return 0,
        };
        let roles: Vec<Role> = manifest_roles.iter().filter_map(|s| role_from_str(s)).collect();
        match Quorum::for_tier(t, &roles) {
            Quorum::AutoApprove => 0,
            Quorum::OneOf(_) => 1,
            Quorum::NofM { n, .. } => n as i32,
            Quorum::Multiset(set) => set.len() as i32,
        }
    }

    /// The approval decision, from the real `Quorum::satisfied_by` — handles
    /// NofM / Multiset and filters non-approving roles (Auditor) itself.
    pub fn quorum_satisfied(tier: &str, required: &[String], signed: &[String]) -> bool {
        let t = match tier_from_str(tier) {
            Some(t) => t,
            None => return false,
        };
        let req: Vec<Role> = required.iter().filter_map(|s| role_from_str(s)).collect();
        let sig: Vec<Role> = signed.iter().filter_map(|s| role_from_str(s)).collect();
        Quorum::for_tier(t, &req).satisfied_by(&sig)
    }
}

/// Real audit-chain integrity verification (STUDIO-3 step 5).
///
/// The studio Audit Scrubber holds a past run's frames in memory; this builds
/// a real `AuditChain` from them (an in-memory sink + the minted genesis) and
/// runs the runtime's own `AuditChain::verify_integrity`. The "Simulate tamper"
/// affordance genuinely corrupts a record's payload so the hash chain breaks —
/// and the verdict + break sequence are exactly what the runtime reports, not a
/// canned string.
pub mod audit {
    use crate::audit_verify::Verdict;
    use citrate_agent_core::audit::record::{AuditRecord, EventType};
    use citrate_agent_core::audit::{AuditChain, AuditSink, GenesisInfo};
    use citrate_agent_core::error::AgentError;
    use std::sync::{Arc, Mutex};

    /// In-memory `AuditSink` — same contract as `FilesystemSink`, no disk.
    struct MemSink {
        records: Mutex<Vec<AuditRecord>>,
    }
    impl MemSink {
        fn new() -> Self {
            Self { records: Mutex::new(Vec::new()) }
        }
    }
    impl AuditSink for MemSink {
        fn append(&self, record: &AuditRecord) -> Result<(), AgentError> {
            self.records.lock().unwrap().push(record.clone());
            Ok(())
        }
        fn iter(
            &self,
        ) -> Result<Box<dyn Iterator<Item = Result<AuditRecord, AgentError>> + '_>, AgentError> {
            let snapshot = self.records.lock().unwrap().clone();
            Ok(Box::new(snapshot.into_iter().map(Ok)))
        }
    }

    fn event_type(evt: &str) -> EventType {
        use EventType::*;
        match evt {
            "Genesis" => Genesis,
            "Proposal" => Proposal,
            "Edit" => Edit,
            "Approval" => Approval,
            "Rejection" => Rejection,
            "Submission" => Submission,
            "Confirmation" => Confirmation,
            "BreakGlass" => BreakGlass,
            "BreakGlassAffirmation" => BreakGlassAffirmation,
            "DoctorReport" => DoctorReport,
            "AuditExport" => AuditExport,
            "Resumption" => Resumption,
            _ => Edit,
        }
    }

    fn err_verdict(msg: &str) -> Verdict {
        // Pull the sequence out of "...break at sequence N: ..." for the UI.
        let break_seq = msg
            .split("sequence ")
            .nth(1)
            .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        Verdict { ok: false, message: msg.to_string(), break_seq }
    }

    /// Build a real `AuditChain` from `(seq, event, actor)` frames and run the
    /// real `verify_integrity`. `tampered` corrupts a record so the chain
    /// genuinely breaks.
    pub fn verify(frames: &[(u64, String, String)], tampered: bool) -> Verdict {
        let sink = Arc::new(MemSink::new());
        let sink_dyn: Arc<dyn AuditSink> = sink.clone();
        let genesis = GenesisInfo {
            agent_did: "did:citrate:core".into(),
            harness_version: "citrate-studio".into(),
            policy_bundle_hash: [0u8; 32],
            doctor_report_hash: [0u8; 32],
        };
        let mut chain = match AuditChain::open_or_init(sink_dyn, genesis, 0) {
            Ok(c) => c,
            Err(e) => return err_verdict(&e.to_string()),
        };
        // open_or_init minted frame 0 (Genesis); append the rest as their events.
        for (_, evt, actor) in frames.iter().skip(1) {
            if let Err(e) =
                chain.append(event_type(evt), evt.as_bytes().to_vec(), actor.clone(), Vec::new(), None, 0)
            {
                return err_verdict(&e.to_string());
            }
        }
        // Corrupt the record just before the break point so verify trips at seq 6.
        if tampered {
            let mut recs = sink.records.lock().unwrap();
            if recs.len() > 5 {
                recs[5].payload = b"TAMPERED".to_vec();
            }
        }
        match chain.verify_integrity() {
            Ok(count) => Verdict {
                ok: true,
                message: format!("verify_integrity() ok · {count} frames"),
                break_seq: -1,
            },
            Err(e) => err_verdict(&e.to_string()),
        }
    }
}

/// Live HITL approval through the REAL async `ApprovalQueue` (STUDIO-6).
///
/// The culmination of STUDIO-4: a studio `EnrolledSigner`'s ed25519 secret is
/// fed to `Ed25519FileSurface::from_seed` to produce an `AttestedSignature` that
/// the runtime's own `verify_attestation` + `StaticSignerRoster` (RM-G.1) +
/// separation-of-duties + `Quorum::satisfied_by` accept — because both sides
/// compute `signer_id = SHA-256(pubkey)` identically. `LiveQueue` wraps the
/// async queue behind a synchronous, UI-thread-friendly surface.
pub mod approvals {
    use citrate_agent_core::capsule::manifest::RiskTier;
    use citrate_agent_core::hitl::{
        signer_id_from_pubkey, ApprovalQueue, Ed25519FileSurface, Quorum, Role, Signature,
        Signer, SigningSurface, StaticSignerRoster, ToolCall,
    };
    use std::sync::Arc;
    use std::time::Duration;

    fn tier_of(s: &str) -> RiskTier {
        match s {
            "low" => RiskTier::Low,
            "medium" => RiskTier::Medium,
            "critical" => RiskTier::Critical,
            _ => RiskTier::High,
        }
    }

    pub struct LiveQueue {
        rt: tokio::runtime::Runtime,
        queue: Arc<ApprovalQueue>,
    }

    impl LiveQueue {
        /// Build the queue with a `StaticSignerRoster` from the enrolled
        /// `(pubkey, role)` pairs — so studio's keys are authorized for real.
        pub fn new(roster: &[([u8; 32], String)]) -> Self {
            let mut sr = StaticSignerRoster::new();
            for (pk, role) in roster {
                if let Some(r) = super::role_from_str(role) {
                    sr = sr.authorize(*pk, r);
                }
            }
            let queue = Arc::new(ApprovalQueue::new().with_signer_roster(Arc::new(sr)));
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .expect("tokio runtime");
            Self { rt, queue }
        }

        /// Open a gate: submit the action to the real queue (async, spawned),
        /// then block briefly until the payload is pinned (registered).
        pub fn open(
            &self,
            call_id: &str,
            name: &str,
            payload: Vec<u8>,
            tier: &str,
            required: &[String],
            proposer_pk: [u8; 32],
            proposer_role: &str,
        ) {
            let req: Vec<Role> = required.iter().filter_map(|r| super::role_from_str(r)).collect();
            let quorum = Quorum::for_tier(tier_of(tier), &req);
            let proposer = Signer {
                id: signer_id_from_pubkey(&proposer_pk),
                role: super::role_from_str(proposer_role).unwrap_or(Role::Operator),
            };
            let call = ToolCall { call_id: call_id.into(), name: name.into(), args: serde_json::json!({}) };
            let q = self.queue.clone();
            self.rt.spawn(async move {
                let _ = q.submit_for_action(call, payload, quorum, proposer).await;
            });
            // The entry is inserted on the future's first poll; wait for it.
            for _ in 0..200 {
                if self.queue.payload_for(call_id).is_some() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        }

        /// Sign the pending action with an enrolled key. The REAL queue verifies
        /// the attestation, authorizes the signer (RM-G.1), enforces SoD, and
        /// decides quorum. Returns `Ok(quorum_met)` or the runtime's rejection.
        pub fn sign(&self, call_id: &str, secret: [u8; 32], role: &str) -> Result<bool, String> {
            let role = super::role_from_str(role).ok_or("unknown role")?;
            let payload = self.queue.payload_for(call_id).ok_or("action not pending")?;
            let surface = Ed25519FileSurface::from_seed(secret, role);
            let sig: Signature = surface.sign(&payload).map_err(|e| e.to_string())?.into();
            self.queue.add_signature(call_id, sig).map_err(|e| format!("{e:?}"))?;
            // quorum met iff the entry resolved Approved and was removed
            Ok(self.queue.payload_for(call_id).is_none())
        }

        pub fn signature_count(&self, call_id: &str) -> usize {
            self.queue.signatures_on(call_id).len()
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn high_gate_satisfied_by_real_enrolled_signatures() {
        // End-to-end: studio's STUDIO-4 keys → real AttestedSignatures → the
        // real async ApprovalQueue's verify + RM-G.1 roster auth + SoD + quorum.
        let roster = crate::signing::Roster::seed_demo();
        let pairs: Vec<([u8; 32], String)> =
            roster.signers.iter().map(|s| (s.pubkey, s.role.clone())).collect();
        let lq = super::approvals::LiveQueue::new(&pairs);

        let op = roster.signer_for("Operator").unwrap();
        lq.open(
            "c3",
            "recon.match-phi",
            b"sha256:7b41...e0c9".to_vec(),
            "high",
            &["Reviewer".into(), "ComplianceOfficer".into(), "SecurityOfficer".into()],
            op.pubkey,
            "Operator",
        );

        let rv = roster.signer_for("Reviewer").unwrap();
        let co = roster.signer_for("ComplianceOfficer").unwrap();
        // One signature: not enough.
        assert!(!lq.sign("c3", rv.secret.unwrap(), "Reviewer").unwrap(), "1 of 2");
        assert_eq!(lq.signature_count("c3"), 1);
        // Two: quorum met through the REAL ApprovalQueue.
        assert!(lq.sign("c3", co.secret.unwrap(), "ComplianceOfficer").unwrap(), "quorum met end-to-end");
    }

    #[test]
    fn real_hello_capsule_dispatches_through_wasmtime() {
        let dir = "../citrate-agent-runtime/capsules";
        let d = match super::dispatch::LiveDispatch::load(dir) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("skipping: capsule fleet not present ({e})");
                return;
            }
        };
        assert!(d.names().contains(&"hello".to_string()), "fleet: {:?}", d.names());
        let out = d.greet("Aleia").expect("greet runs");
        assert!(!out.is_empty(), "real ToolResult from wasmtime");
        eprintln!("hello capsule returned: {out}");
    }
}

/// Real capsule dispatch through wasmtime (STUDIO-6).
///
/// Points the runtime's `CapsuleDispatch` at the prebuilt capsule fleet in
/// `citrate-agent-runtime/capsules/` and executes a real WASM component. The
/// `hello` capsule's `greeter.greet(name)` is pure compute (no host imports),
/// so it round-trips a real `ToolResult` with no chain/fs needed.
pub mod dispatch {
    use citrate_agent_core::capsule::dispatch::CapsuleDispatch;
    use std::path::Path;
    use wasmtime::component::Val;

    pub struct LiveDispatch {
        inner: CapsuleDispatch,
    }

    impl LiveDispatch {
        /// Load every capsule (subdir with manifest.toml + capsule.wasm) from `dir`.
        pub fn load(dir: &str) -> Result<Self, String> {
            let inner = CapsuleDispatch::load_from_dir(Path::new(dir), None, None, None)
                .map_err(|e| e.to_string())?;
            Ok(Self { inner })
        }

        /// Names of the loaded fleet.
        pub fn names(&self) -> Vec<String> {
            self.inner.capsule_names()
        }

        /// Dispatch the `hello` capsule's `greet` — real wasmtime execution.
        /// The shipped capsules carry placeholder content-hashes (signing is
        /// CIT-AGENT-3e), so running them is a loudly-logged dev opt-in.
        pub fn greet(&self, name: &str) -> Result<String, String> {
            std::env::set_var("CITRATE_ALLOW_UNVERIFIED_CAPSULES", "1");
            // the interface is exported package-qualified (wasmtime component model)
            let out = self
                .inner
                .call_raw(
                    "hello",
                    "citrate:hello-capsule/greeter@0.1.0",
                    "greet",
                    &[Val::String(name.to_string())],
                )
                .map_err(|e| e.to_string())?;
            match out {
                Val::String(s) => Ok(s),
                other => Err(format!("unexpected return: {other:?}")),
            }
        }
    }
}
