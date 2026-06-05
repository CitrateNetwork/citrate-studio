// Citrate Studio — demo data (mirrors studio-data.js).
// Scenario SOP-RECON-NIGHTLY: snapshot ledger → pull CUI → cross-match
// PHI (HIGH quorum gate) → write report → anchor Merkle root.
// TRIP-AU-002 (audit-shipper backlog) fires mid-run.
use crate::{
    CapsuleData, ClipData, DoctorCheck, FrameData, Member, OnboardStep, ToolData,
    Tripwire,
};
use slint::{ModelRc, SharedString, VecModel};

fn ss(v: &[&str]) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        v.iter().map(|s| SharedString::from(*s)).collect::<Vec<_>>(),
    ))
}

fn net_label(caps: &[&str]) -> &'static str {
    if caps.contains(&"net") { "egress-allowed" } else { "none" }
}
fn fs_label(caps: &[&str], writes: &[&str]) -> &'static str {
    if !caps.contains(&"fs") {
        "—"
    } else if writes.is_empty() {
        "read:/tenant/*"
    } else {
        "read|write:/tenant/*"
    }
}

#[allow(clippy::too_many_arguments)]
fn clip(
    id: &str, name: &str, version: &str, risk: &str, gate: &str,
    caps: &[&str], reads: &[&str], writes: &[&str], emits: &[&str],
    start: i32, dur: i32, purpose: &str, output: &str, data_json: &str,
    publisher: &str, reproducible: bool, sprint: &str, contracts: &[&str],
    quorum_n: i32, quorum_roles: &[&str], payload_hash: &str,
) -> ClipData {
    ClipData {
        id: id.into(), name: name.into(), version: version.into(),
        risk: risk.into(), gate: gate.into(),
        caps: ss(caps), reads: ss(reads), writes: ss(writes), emits: ss(emits),
        start, dur, purpose: purpose.into(), output: output.into(),
        data_json: data_json.into(), publisher: publisher.into(),
        reproducible, sprint: sprint.into(), contracts: ss(contracts),
        quorum_n, quorum_roles: ss(quorum_roles), payload_hash: payload_hash.into(),
        dimmed: false,
        network: net_label(caps).into(),
        filesystem: fs_label(caps, writes).into(),
    }
}

pub fn clips() -> Vec<ClipData> {
    vec![
        clip("c1", "recon.snapshot", "2.4.1", "low", "auto",
            &["read", "clocks"], &["PUBLIC"], &[], &["PUBLIC"], 0, 8,
            "Snapshot ledger heads across all tenant facilities.",
            "Snapshotted 1,284 ledger heads across 9 facilities.",
            "{\n facilities: 9,\n heads: 1284,\n ms: 412\n}",
            "did:citrate:agent:0xab12", true, "AGT-09",
            &["eth_call:0x5e…AnchorRegistry"], 0, &[], ""),
        clip("c2", "recon.fetch-records", "1.9.0", "medium", "medium",
            &["fs", "clocks"], &["CUI"], &[], &["CUI"], 8, 10,
            "Pull CUI-classified procurement records, read-only.",
            "Pulled 3,402 CUI procurement records (read-only).",
            "{\n records: 3402,\n classes: [CUI],\n paths: [read:/tenant/procurement]\n}",
            "did:citrate:agent:0x77c4", true, "AGT-11", &[], 0, &[], ""),
        clip("c3", "recon.match-phi", "0.8.3", "high", "high",
            &["fs", "clocks", "random"], &["PHI", "CUI"], &[], &["CUI"], 18, 15,
            "Cross-match procurement against PHI custody records; flag anomalies.",
            "Cross-matched 612 custody records; 7 anomalies flagged.",
            "{\n matched: 612,\n anomalies: 7,\n classes: [PHI, CUI]\n}",
            "did:citrate:agent:0x9f4a", false, "AGT-13", &[],
            2, &["Reviewer", "ComplianceOfficer", "SecurityOfficer"], "sha256:7b41…e0c9"),
        clip("c4", "recon.write-report", "1.2.0", "medium", "medium",
            &["fs"], &["CUI"], &["CUI"], &["CUI"], 33, 9,
            "Write the reconciliation report (denies secret paths).",
            "Wrote recon-2026-06-03.cui.md — secrets paths denied.",
            "{\n file: recon-2026-06-03.cui.md,\n bytes: 48213,\n denied: [.env, *.pem]\n}",
            "did:citrate:agent:0x77c4", true, "AGT-11", &[], 0, &[], ""),
        clip("c5", "recon.anchor-merkle", "3.0.2", "low", "auto",
            &["read", "clocks"], &["PUBLIC"], &[], &["PUBLIC"], 42, 10,
            "Anchor the run's Merkle root on chain 40204 (NightlyMerkle).",
            "Anchored Merkle root 0x4c9f… via NightlyMerkle on chain 40204.",
            "{\n strategy: NightlyMerkle,\n chain: 40204,\n root: 0x4c9f…2a1b\n}",
            "did:citrate:core", true, "AGT-07",
            &["eth_send:0x5e…AnchorRegistry"], 0, &[], ""),
    ]
}

fn tool(name: &str, risk: &str, cap: &str, nature: &str) -> ToolData {
    ToolData { name: name.into(), risk: risk.into(), cap: cap.into(), nature: nature.into() }
}

pub fn tools_chain() -> Vec<ToolData> {
    vec![
        tool("check_balance", "low", "read", "chain-read"),
        tool("query_contract", "low", "read", "chain-read · eth_call"),
        tool("list_models", "low", "cli", "read"),
        tool("run_inference", "low", "cli", "read · local GGUF"),
        tool("explain_tx", "low", "read", "chain-read"),
        tool("get_block_height", "low", "read", "chain-read"),
        tool("get_peer_count", "low", "read", "chain-read"),
        tool("get_recent_blocks", "low", "read", "chain-read"),
        tool("get_tx_history", "low", "read", "chain-read"),
        tool("send_tx", "high", "write", "chain-write · approval"),
        tool("deploy_contract", "high", "write", "chain-write · approval"),
    ]
}
pub fn tools_code() -> Vec<ToolData> {
    vec![
        tool("file_read", "low", "fs", "fs-read · workspace"),
        tool("search_code", "low", "fs", "fs-read"),
        tool("file_write", "medium", "fs", "fs-write · denies secrets"),
        tool("file_edit", "medium", "fs", "fs-write · exact-match"),
        tool("git_ops", "medium", "cli", "execute · commit/push"),
        tool("shell_exec", "critical", "cli", "execute · allowlist only"),
    ]
}

fn capsule(name: &str, version: &str, risk: &str, caps: &[&str], signing: &str, verified: bool, dc: &str, hash: &str) -> CapsuleData {
    CapsuleData {
        name: name.into(), version: version.into(), risk: risk.into(),
        caps: ss(caps), signing: signing.into(), verified, dc: dc.into(), hash: hash.into(),
    }
}
pub fn capsules() -> Vec<CapsuleData> {
    vec![
        capsule("recon.match-phi", "0.8.3", "high", &["fs", "clocks", "random"], "managed", true, "", ""),
        capsule("recon.snapshot", "2.4.1", "low", &["read", "clocks"], "bundled", true, "", ""),
        capsule("custody.attest", "1.1.0", "high", &["read", "write"], "workspace", true, "ITAR", ""),
        capsule("export.itar-pack", "0.4.0", "critical", &["fs", "net"], "workspace", false, "ITAR", "sha256:000…0"),
        capsule("vendor.unsigned-scan", "0.0.7", "medium", &["fs"], "—", false, "", "sha256:000…0"),
    ]
}

fn frame(seq: i32, evt: &str, actor: &str, role: &str, anchor: &str, surface: &str) -> FrameData {
    FrameData { seq, evt: evt.into(), actor: actor.into(), role: role.into(), anchor: anchor.into(), surface: surface.into() }
}
pub fn frames() -> Vec<FrameData> {
    vec![
        frame(0, "Genesis", "did:citrate:core", "", "NightlyMerkle", "FileBacked"),
        frame(1, "Proposal", "did:citrate:agent:0x9f4a", "", "", "Slint"),
        frame(2, "Edit", "did:citrate:op:aleia", "", "", "Slint"),
        frame(3, "Approval", "did:citrate:rev:dorian", "Reviewer", "PerApproval", "Slint"),
        frame(4, "Approval", "did:citrate:co:priya", "ComplianceOfficer", "PerApproval", "LocalWeb"),
        frame(5, "Submission", "did:citrate:core", "", "", "FileBacked"),
        frame(6, "Confirmation", "did:citrate:core", "", "NightlyMerkle", "FileBacked"),
        frame(7, "BreakGlass", "did:citrate:so:marcus", "SecurityOfficer", "", "Mobile"),
        frame(8, "BreakGlassAffirmation", "did:citrate:co:priya", "ComplianceOfficer", "", "LocalWeb"),
        frame(9, "DoctorReport", "did:citrate:core", "", "", "Cli"),
        frame(10, "AuditExport", "did:citrate:aud:ext", "Auditor", "NightlyMerkle", "FileBacked"),
        frame(11, "Resumption", "did:citrate:core", "", "", "FileBacked"),
    ]
}

fn dcheck(id: &str, sev: &str, note: &str) -> DoctorCheck {
    DoctorCheck { id: id.into(), sev: sev.into(), note: note.into() }
}
pub fn doctor() -> Vec<DoctorCheck> {
    vec![
        dcheck("audit-chain-integrity", "Pass", "verify_integrity ok · 11,402 records"),
        dcheck("audit-file-permissions", "Pass", "mode 0o600"),
        dcheck("approval-queue-depth", "Pass", "3 / 100"),
        dcheck("pending-break-glass", "Pass", "none"),
        dcheck("runtime-presence", "Pass", "tokio ok"),
        dcheck("capsule-manifest-reverify", "Pass", "9 capsules re-verified"),
        dcheck("wasm-linker-recheck", "Pass", "manifest↔WIT match"),
        dcheck("policy-bundle-hash", "Pass", "matches pinned"),
        dcheck("tla-spec-ci-status", "Pass", "35,435 states · green"),
        dcheck("retention-age", "Pass", "mtime 6d / 90d"),
        dcheck("anchor-reconciliation", "Warn", "2 roots pending reconciliation"),
    ]
}

fn trip(id: &str, code: &str, cond: &str, sev: &str, state: &str, tx: &str) -> Tripwire {
    Tripwire { id: id.into(), code: code.into(), cond: cond.into(), sev: sev.into(), state: state.into(), tx: tx.into() }
}
pub fn tripwires() -> Vec<Tripwire> {
    vec![
        trip("TRIP-AU-001", "AU", "RocksDB hot tier > 80%", "medium", "NoBreach", ""),
        trip("TRIP-AU-002", "AU", "Audit shipper backlog > 1000", "high", "Fired", "0x8c12…fired"),
        trip("TRIP-SC-001", "SC", "TLS cert expiry < 30 days", "medium", "NoBreach", ""),
        trip("TRIP-AC-001", "AC", "Account elevated > 4h continuous", "high", "NoBreach", ""),
        trip("TRIP-CM-001", "CM", "Multi-sig method by non-multi-sig acct", "high", "NoBreach", ""),
        trip("TRIP-IA-001", "IA", "requestElevation without auth_mode", "medium", "NoBreach", ""),
        trip("TRIP-AC-002", "AC", "Status change → no auto-revoke in 5 blocks", "critical", "NoBreach", ""),
        trip("TRIP-AU-003", "AU", "IPFS pin failure rate > 1%/hour", "critical", "NoBreach", ""),
        trip("TRIP-SI-001", "SI", "Release manifest hash mismatch on deploy", "critical", "NoBreach", ""),
    ]
}

// Signers now come from the real enrolled ed25519 roster (STUDIO-4):
// see `signing::Roster` + `signers_from_roster` in main.rs.

// ---- RBAC directory + initial team (settings.jsx DIRECTORY) ----
fn member(id: &str, name: &str, title: &str, role: &str, team_role: &str) -> Member {
    Member {
        id: id.into(), name: name.into(), title: title.into(),
        role: role.into(), team_role: team_role.into(),
    }
}
pub fn directory() -> Vec<Member> {
    vec![
        member("aleia", "Aleia Rouhani", "Compliance lead", "ComplianceOfficer", ""),
        member("dorian", "Dorian Vale", "Staff engineer", "Reviewer", ""),
        member("priya", "Priya Anand", "Compliance officer", "ComplianceOfficer", ""),
        member("marcus", "Marcus Greel", "Security officer", "SecurityOfficer", ""),
        member("ext", "Ext. Auditor", "Independent (read-only)", "Auditor", ""),
        member("sam", "Sam Okafor", "Operator", "Operator", ""),
        member("lena", "Lena Fischer", "Operator", "Operator", ""),
        member("raj", "Raj Mehta", "Reviewer", "Reviewer", ""),
    ]
}
pub fn rbac_initial() -> Vec<Member> {
    vec![
        member("aleia", "Aleia Rouhani", "Compliance lead", "ComplianceOfficer", "Admin"),
        member("dorian", "Dorian Vale", "Staff engineer", "Reviewer", "Member"),
        member("priya", "Priya Anand", "Compliance officer", "ComplianceOfficer", "Member"),
        member("marcus", "Marcus Greel", "Security officer", "SecurityOfficer", "Member"),
        member("ext", "Ext. Auditor", "Independent (read-only)", "Auditor", "Member"),
    ]
}

// ---- onboarding steps (onboarding.jsx STEPS) ----
fn ostep(id: &str, label: &str, icon: &str, active: bool) -> OnboardStep {
    OnboardStep {
        id: id.into(), label: label.into(), icon: icon.into(),
        value: "".into(), done: false, active,
    }
}
pub fn onboard_steps() -> Vec<OnboardStep> {
    vec![
        ostep("workspace", "Workspace", "building", true),
        ostep("runtime", "Model runtime", "spark", false),
        ostep("roster", "Approval roster · RBAC", "users", false),
        ostep("capsules", "Capsules", "layers", false),
        ostep("oversight", "Oversight default", "gavel", false),
        ostep("prompt", "First prompt", "chat", false),
    ]
}
