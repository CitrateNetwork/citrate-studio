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
}
