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
