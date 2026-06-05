// Citrate Studio — native shell entry.
//
// The front end is a viewport + intent submitter (per the design
// spec's trust boundary): it renders state and submits intents; the
// Rust "core" (modeled here) decides quorum/approval/SoD. The UI
// never computes "approved".
//
// Fonts are embedded at compile time via `import "*.ttf"` in
// ui/studio.slint, so no runtime font registration is needed.

mod data;
// STUDIO-2 auth scaffold — pure for now; the UI sign-in surface + network
// client land in a later STUDIO-2 commit. `allow(dead_code)` until wired.
#[allow(dead_code)]
mod auth;
// STUDIO-3 — bridge to the real citrate-agent-core (only under `core-live`).
#[cfg(feature = "core-live")]
mod core_bridge;

/// Policy seam — separation-of-duties + quorum. The default build keeps a
/// faithful hand-rolled copy of the runtime's rules; the `core-live` feature
/// swaps in the authoritative `citrate-agent-core` (see core_bridge.rs).
/// Callers compute through `policy::*` and never know which is compiled — the
/// concrete proof that wiring the real core is a swap, not a rewrite.
mod policy {
    #[cfg(not(feature = "core-live"))]
    pub fn is_conflict(a: &str, b: &str) -> bool {
        matches!(
            (a, b),
            ("ComplianceOfficer", "SecurityOfficer") | ("SecurityOfficer", "ComplianceOfficer")
        )
    }
    #[cfg(not(feature = "core-live"))]
    pub fn can_approve(role: &str) -> bool {
        role != "Auditor"
    }
    #[cfg(not(feature = "core-live"))]
    pub fn quorum_n(tier: &str, _manifest_roles: &[String]) -> i32 {
        match tier {
            "medium" => 1,
            "high" => 2,
            "critical" => 3,
            _ => 0,
        }
    }
    /// The approval decision itself — is the quorum satisfied by these signed
    /// roles? Mirrors the core's `Quorum::satisfied_by`: NofM needs n hits from
    /// the required set; Critical needs the fixed {SO, CO, Reviewer} multiset;
    /// non-approving roles (Auditor) never count.
    #[cfg(not(feature = "core-live"))]
    pub fn quorum_satisfied(tier: &str, required: &[String], signed: &[String]) -> bool {
        let usable: Vec<&String> = signed.iter().filter(|r| can_approve(r)).collect();
        match tier {
            "low" => true,
            "medium" => usable.iter().any(|r| required.contains(r)),
            "high" => usable.iter().filter(|r| required.contains(r)).count() >= 2,
            "critical" => ["SecurityOfficer", "ComplianceOfficer", "Reviewer"]
                .iter()
                .all(|need| usable.iter().any(|r| r.as_str() == *need)),
            _ => false,
        }
    }
    #[cfg(feature = "core-live")]
    pub use crate::core_bridge::policy::{can_approve, is_conflict, quorum_n, quorum_satisfied};
}

/// Audit-chain verification seam. The scrubber's verdict (verify ok / count,
/// or the broken-link sequence on tamper) comes from here: the default build
/// models it; `core-live` runs the real `AuditChain::verify_integrity` over a
/// real in-memory chain built from the frames (see core_bridge::audit). Both
/// must agree — the parity test guards it.
mod audit_verify {
    #[derive(Clone)]
    pub struct Verdict {
        pub ok: bool,
        pub message: String, // carries the verified frame count when ok
        pub break_seq: i32,  // frame sequence where the chain breaks; -1 = clean
    }
    #[cfg(not(feature = "core-live"))]
    pub fn verify(frames: &[(u64, String, String)], tampered: bool) -> Verdict {
        if tampered {
            Verdict {
                ok: false,
                message: "previous_hash break at sequence 6: chain tampered".into(),
                break_seq: 6,
            }
        } else {
            let n = frames.len();
            Verdict {
                ok: true,
                message: format!("verify_integrity() ok · {n} frames"),
                break_seq: -1,
            }
        }
    }
    #[cfg(feature = "core-live")]
    pub use crate::core_bridge::audit::verify;
}

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

const SPEED: f32 = 0.62; // timeline units per tick
const TICK_MS: u64 = 55;
const UNITS_TOTAL: f32 = 56.0;

/// Canonical run state — the "core" the UI submits intents to.
struct RunState {
    base: Vec<ClipData>, // immutable clip facts for lookups
    signers: Vec<Signer>,
    playhead: f32,
    status: String, // idle | running | paused | done
    pending: Option<String>,
    signatures: Vec<(String, String, String)>, // (role, name, surface)
    approved: Vec<String>,
    medium: Vec<String>, // clip ids queued
    auto: Vec<String>,   // capsule names auto-approved
    trip_fired: bool,
    dry: Vec<String>,
    solo: Option<String>,
    oversight: String, // in | on | out
    ttl: i32,
    // chat (beginner + drawer)
    chat: Vec<ChatMsg>,
    chat_quick: Vec<String>,
    chat_busy: bool,
    // settings / rbac
    rbac: Vec<Member>,
    rbac_dropped: Vec<String>,
    // onboarding
    ob_steps: Vec<OnboardStep>,
    ob_msgs: Vec<ChatMsg>,
    ob_quick: Vec<String>,
    ob_ready: bool,
    ob_team: bool,
}

impl RunState {
    fn clip(&self, id: &str) -> Option<ClipData> {
        self.base.iter().find(|c| c.id == id).cloned()
    }
    fn clip_by_name(&self, name: &str) -> Option<ClipData> {
        self.base.iter().find(|c| c.name == name).cloned()
    }
    fn quorum_met(&self) -> bool {
        if let Some(id) = &self.pending {
            if let Some(c) = self.clip(id) {
                // The approval decision is the core's: Quorum::satisfied_by
                // (under core-live), not a signature count in the UI.
                let required = roles_of(&c);
                let signed: Vec<String> = self.signatures.iter().map(|s| s.0.clone()).collect();
                return policy::quorum_satisfied(&c.risk, &required, &signed);
            }
        }
        false
    }
    fn reset(&mut self) {
        self.playhead = 0.0;
        self.status = "idle".into();
        self.pending = None;
        self.signatures.clear();
        self.approved.clear();
        self.medium.clear();
        self.auto.clear();
        self.trip_fired = false;
    }

    /// Advance the run by one playback tick (assumes `status == "running"`).
    /// Extracted from the timer closure so the run state machine — gate
    /// pause, medium-queue / auto-feed crossings, tripwire fire, completion —
    /// is unit-testable. Mirrors shell.jsx's loop.
    fn tick(&mut self) {
        let prev = self.playhead;
        let mut next = prev + SPEED;
        let base = self.base.clone();
        // gate pause (high/critical)
        for c in &base {
            if (c.gate == "high" || c.gate == "critical")
                && !self.approved.contains(&c.id.to_string())
            {
                let cs = c.start as f32;
                if next >= cs && prev <= cs + 0.001 {
                    next = cs;
                    self.status = "paused".into();
                    self.pending = Some(c.id.to_string());
                    self.signatures.clear();
                    break;
                }
            }
        }
        // side-effects on crossing boundaries
        for c in &base {
            let cs = c.start as f32;
            let ce = (c.start + c.dur) as f32;
            if c.gate == "medium" && prev < cs && next >= cs && !self.medium.contains(&c.id.to_string()) {
                self.medium.push(c.id.to_string());
            }
            if c.gate == "auto" && prev < ce && next >= ce && !self.auto.contains(&c.name.to_string()) {
                self.auto.push(c.name.to_string());
            }
        }
        if !self.trip_fired && next >= 22.0 {
            self.trip_fired = true;
        }
        if next >= UNITS_TOTAL {
            next = UNITS_TOTAL;
            self.status = "done".into();
        }
        self.playhead = next;
    }
}

fn vm<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(v))
}

fn roles_of(c: &ClipData) -> Vec<String> {
    c.quorum_roles.iter().map(|s| s.to_string()).collect()
}

fn join_q(m: &slint::ModelRc<SharedString>) -> String {
    m.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", ")
}

/// Generate the L4 source views for a capsule (manifest / WIT / calldata).
fn gen_source(c: &ClipData) -> (String, String, String) {
    let caps: Vec<String> = c.caps.iter().map(|s| s.to_string()).collect();
    let has = |k: &str| caps.iter().any(|x| x == k);
    let reads: Vec<String> = c.reads.iter().map(|s| s.to_string()).collect();
    let itar = reads.iter().any(|r| r == "ITAR");
    let net = if has("net") { "egress-allowed" } else { "none" };
    let fs = if has("fs") {
        if c.writes.iter().count() > 0 { "\"read|write:/tenant\"" } else { "\"read:/tenant\"" }
    } else { "" };
    let roles = if c.quorum_roles.iter().count() > 0 { join_q(&c.quorum_roles) } else { "\"Operator\"".into() };

    let manifest = format!(
"# manifest.toml — round-trips to Manifest (manifest.rs:33)
[metadata]
name = \"{name}\"
version = \"{ver}\"

[risk]
tier = \"{risk}\"
required_roles = [{roles}]
break_glass_eligible = {bge}

[capabilities]
network = \"{net}\"
filesystem = [{fs}]
chain_calls = [{calls}]
subagent_spawn = false   # v1 always false

[data_classes]
reads  = [{reads}]
writes = [{writes}]
emits  = [{emits}]

[provenance]
publisher = \"{pub}\"
build_reproducible = {repro}
agentile_sprint = \"{sprint}\"",
        name = c.name, ver = c.version, risk = c.risk, roles = roles, bge = !itar,
        net = net, fs = fs, calls = join_q(&c.contracts),
        reads = join_q(&c.reads), writes = join_q(&c.writes), emits = join_q(&c.emits),
        pub = c.publisher, repro = c.reproducible, sprint = c.sprint,
    );

    let world = c.name.replace('.', "-");
    let mut wit = format!("package citrate:capsule@{ver};\n\nworld {world} {{\n", ver = c.version, world = world);
    if has("fs") { wit.push_str("  import wasi:filesystem/preopens@0.2.0;   // ✓ matches filesystem\n"); }
    if has("clocks") { wit.push_str("  import wasi:clocks/wall-clock@0.2.0;     // ✓ matches clocks\n"); }
    if has("random") { wit.push_str("  import wasi:random/random@0.2.0;         // ✓ matches random\n"); }
    if has("read") || has("write") { wit.push_str("  import citrate:chain/eth@0.2.0;          // ✓ matches chain_calls\n"); }
    wit.push_str("\n  export run: func(input: list<u8>) -> result<list<u8>, string>;\n}");

    let calldata = if c.contracts.iter().count() > 0 {
        let mut s = String::from("// chain_calls allow-list:\n");
        for ct in c.contracts.iter() {
            s.push_str(&format!("//   {ct}\n"));
        }
        if c.contracts.iter().any(|x| x.starts_with("eth_send")) {
            s.push_str("\n// DECODED CALLDATA · chain 40204 · byte-addressed\nselector  0x4a8c3d11   anchorMerkle(bytes32,uint64)\n0x00..20  4c9f2a1b…  root\n0x20..28  000000a4   epoch = 164");
        }
        s
    } else {
        "// no chain_calls declared — this capsule touches no calldata.".into()
    };
    (manifest, wit, calldata)
}

/// Compute the approval roster (SoD owned by the core).
fn approval_roster(st: &RunState) -> Vec<ApprovalRow> {
    let pending = match &st.pending {
        Some(id) => match st.clip(id) {
            Some(c) => c,
            None => return vec![],
        },
        None => return vec![],
    };
    let gate_roles = roles_of(&pending);
    let signed: Vec<String> = st.signatures.iter().map(|s| s.0.clone()).collect();
    let met = st.quorum_met();

    st.signers
        .iter()
        .filter(|s| gate_roles.contains(&s.role.to_string()))
        .map(|s| {
            let role = s.role.to_string();
            let is_signed = signed.contains(&role);
            let surface = st
                .signatures
                .iter()
                .find(|x| x.0 == role)
                .map(|x| x.2.clone())
                .unwrap_or_default();
            // SoD + approvability come from policy (the real core under
            // `core-live`). A candidate is conflicted if any already-signed
            // role conflicts with it (CO ⊥ SO).
            let reason: Option<&str> = if s.proposer {
                Some("Proposer can't self-approve")
            } else if s.readonly || !policy::can_approve(&role) {
                Some("Auditor never approves")
            } else if is_signed {
                Some("Signed")
            } else if signed.iter().any(|sr| policy::is_conflict(&role, sr)) {
                Some("SoD · CO ⊥ SO")
            } else if met {
                Some("Quorum met")
            } else {
                None
            };
            ApprovalRow {
                role: s.role.clone(),
                name: s.name.clone(),
                fp: s.fp.clone(),
                signed: is_signed,
                surface: surface.into(),
                disabled: reason.is_some() && !is_signed,
                reason: reason.unwrap_or("").into(),
            }
        })
        .collect()
}

fn derive_depth(ui: &StudioWindow, st: &RunState) -> &'static str {
    let app = ui.global::<AppState>();
    if !app.get_code_clip().is_empty() {
        "L4"
    } else if app.get_has_selected() {
        "L2"
    } else if app.get_health_open() || app.get_breakglass_open() || st.pending.is_some() {
        "L3"
    } else if app.get_l0_visible() && st.status == "idle" {
        "L0"
    } else {
        "L1"
    }
}

/// Push the full RunState into the AppState global (recompute derived).
fn refresh(ui: &StudioWindow, st: &RunState) {
    let app = ui.global::<AppState>();

    // dimmed-applied clips
    let clips: Vec<ClipData> = st
        .base
        .iter()
        .map(|c| {
            let mut c = c.clone();
            c.dimmed = st.dry.iter().any(|d| d.as_str() == c.id.as_str())
                || st.solo.as_ref().map(|s| s.as_str() != c.id.as_str()).unwrap_or(false);
            c
        })
        .collect();
    app.set_clips(vm(clips));

    app.set_playhead(st.playhead);
    app.set_status(st.status.clone().into());
    app.set_pending_gate_id(st.pending.clone().unwrap_or_default().into());
    app.set_trip_fired(st.trip_fired);
    app.set_oversight(st.oversight.clone().into());
    app.set_ttl(st.ttl);

    // signatures
    let sigs: Vec<SigRow> = st
        .signatures
        .iter()
        .map(|(role, name, surface)| SigRow {
            role: role.into(),
            name: name.into(),
            surface: surface.into(),
        })
        .collect();
    app.set_signatures(vm(sigs));

    // pending / quorum
    let pending_clip = st.pending.as_ref().and_then(|id| st.clip(id));
    let is_high = pending_clip
        .as_ref()
        .map(|c| c.gate == "high" || c.gate == "critical")
        .unwrap_or(false);
    app.set_has_pending(is_high);
    app.set_pending_clip(pending_clip.clone().unwrap_or_default());
    app.set_quorum_need(
        pending_clip
            .as_ref()
            .map(|c| policy::quorum_n(&c.risk, &roles_of(c)))
            .unwrap_or(0),
    );
    app.set_quorum_met(st.quorum_met());
    app.set_approval_roster(vm(approval_roster(st)));

    // medium queue → clip data
    let mediums: Vec<ClipData> = st
        .medium
        .iter()
        .filter_map(|id| st.clip(id))
        .collect();
    app.set_medium_clips(vm(mediums));

    // auto feed
    let feed: Vec<SharedString> = st.auto.iter().map(|n| n.into()).collect();
    app.set_auto_feed(vm(feed));

    // chat
    app.set_chat_messages(vm(st.chat.clone()));
    app.set_chat_quick(vm(st.chat_quick.iter().map(|s| s.into()).collect::<Vec<SharedString>>()));
    app.set_chat_busy(st.chat_busy);

    // settings / rbac
    app.set_rbac_members(vm(st.rbac.clone()));
    let in_team: std::collections::HashSet<String> = st.rbac.iter().map(|m| m.id.to_string()).collect();
    let addable: Vec<Member> = data::directory().into_iter().filter(|p| !in_team.contains(p.id.as_str())).collect();
    app.set_rbac_addable(vm(addable));
    app.set_rbac_dropped(vm(st.rbac_dropped.iter().map(|s| s.into()).collect::<Vec<SharedString>>()));

    // onboarding
    app.set_onboard_steps(vm(st.ob_steps.clone()));
    app.set_onboard_messages(vm(st.ob_msgs.clone()));
    app.set_onboard_quick(vm(st.ob_quick.iter().map(|s| s.into()).collect::<Vec<SharedString>>()));
    let done = st.ob_steps.iter().filter(|s| s.done).count() as i32;
    app.set_onboard_progress(done * 100 / st.ob_steps.len().max(1) as i32);
    app.set_onboard_ready(st.ob_ready);
    app.set_onboard_team(st.ob_team);

    // depth
    let d = derive_depth(ui, st);
    app.set_depth(d.into());
}

fn new_state() -> Rc<RefCell<RunState>> {
    Rc::new(RefCell::new(RunState {
        base: data::clips(),
        signers: data::signers(),
        playhead: 0.0,
        status: "idle".into(),
        pending: None,
        signatures: vec![],
        approved: vec![],
        medium: vec![],
        auto: vec![],
        trip_fired: false,
        dry: vec![],
        solo: None,
        oversight: "in".into(),
        ttl: 3600,
        chat: vec![amsg("You're set up, Aleia. Tell me an outcome and I'll handle the capsules, route the approvals, and write every step to the sealed ledger. What should I do first?")],
        chat_quick: svec(&["Run the nightly reconciliation", "What can you do?", "Show me a past run"]),
        chat_busy: false,
        rbac: data::rbac_initial(),
        rbac_dropped: vec![],
        ob_steps: data::onboard_steps(),
        ob_msgs: vec![amsg("Welcome, Aleia. I'll have you prompting in about a minute — and you'll watch every choice land on the right. First: is this workspace for just you, or a team?")],
        ob_quick: svec(&["Just me", "A team"]),
        ob_ready: false,
        ob_team: false,
    }))
}

fn svec(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

// ---- chat message builders ----
fn amsg(text: &str) -> ChatMsg {
    ChatMsg { kind: "agent".into(), text: text.into(), card_title: "".into(), steps: vm(vec![]), cta: "".into() }
}
fn umsg(text: &str) -> ChatMsg {
    ChatMsg { kind: "user".into(), text: text.into(), card_title: "".into(), steps: vm(vec![]), cta: "".into() }
}
fn approval_msg() -> ChatMsg {
    ChatMsg { kind: "approval".into(), text: "".into(), card_title: "".into(), steps: vm(vec![]), cta: "".into() }
}
fn cstep(name: &str, risk: &str, note: &str) -> CardStep {
    CardStep { name: name.into(), risk: risk.into(), note: note.into() }
}

/// In-character local agent reply (no live model). Ported from chat.jsx.
fn local_reply(t: &str) -> &'static str {
    if t.contains("hello") || t.contains("hi ") || t.contains("hey") || t.contains("start") {
        "I'm your Citrate agent. Tell me the outcome you want — I'll assemble the capsules, route any approvals through your roster, and write every step to the sealed ledger."
    } else if t.contains("reconcil") || t.contains("nightly") || t.contains("ledger") {
        "I can run the nightly reconciliation: snapshot ledger heads, pull the CUI records, cross-match PHI custody (that one needs 2-of-N quorum), write the report, and anchor a Merkle root. Want me to stage it?"
    } else if t.contains("approv") || t.contains("quorum") || t.contains("sign") {
        "Approvals are gated by risk tier — Low auto-approves silently, High needs 2-of-N from your roster, Critical is a full-stop. I submit the intent; the core decides. Nothing here grants itself permission."
    } else if t.contains("phi") || t.contains("cui") || t.contains("itar") || t.contains("data") || t.contains("anomal") {
        "Each capsule declares the data classes it reads, writes, and emits. ITAR is hard-blocked from break-glass. I'll only touch a class a capsule has declared and your policy allows."
    } else if t.contains("audit") || t.contains("replay") || t.contains("tamper") || t.contains("proof") {
        "Every action is written to an append-only, hash-chained record and anchored on-chain. You can scrub any past run frame-by-frame; tampering shows as a broken link at the exact sequence."
    } else if t.contains("capsule") || t.contains("tool") || t.contains("build") {
        "Capsules are signed, versioned tools. Unsigned or hash-mismatched ones stay locked — I won't stage what the runtime would refuse to run. I can open the Code Drawer to build one."
    } else {
        "Understood. I'll keep this on the record — say the word and I'll stage it, or open Advanced to watch the composition run layer by layer."
    }
}

/// Beginner/drawer chat: append the user turn, script the agent's reply,
/// and (for "run") set up the inline HITL gate the core will resolve.
fn handle_chat(w: &Weak<StudioWindow>, st: &Rc<RefCell<RunState>>, text: &str) {
    let mut open_adv = false;
    {
        let mut s = st.borrow_mut();
        s.chat.push(umsg(text));
        s.chat_quick.clear();
        let t = text.to_lowercase();
        let run_re = ["run", "reconcil", "stage", "do it", " yes", "go ", "start"]
            .iter().any(|k| t.contains(k)) || t == "yes" || t == "go";
        let watch_re = ["watch", "advanced", "show", "studio", "past run"].iter().any(|k| t.contains(k));
        if watch_re {
            open_adv = true;
        } else if run_re {
            s.chat.push(amsg("Staging the nightly reconciliation. The Low and Medium steps clear automatically — the PHI cross-match is High, so it needs 2-of-N from your roster. Sign right here:"));
            s.pending = Some("c3".into());
            s.signatures.clear();
            s.chat.push(approval_msg());
        } else {
            s.chat.push(amsg(local_reply(&t)));
        }
    }
    if open_adv {
        if let Some(ui) = w.upgrade() {
            let app = ui.global::<AppState>();
            app.set_workspace("advanced".into());
            app.set_chat_open(false);
            {
                let mut s = st.borrow_mut();
                s.reset();
                s.status = "running".into();
            }
        }
    }
    if let Some(ui) = w.upgrade() {
        refresh(&ui, &st.borrow());
    }
}

/// Conversational onboarding: advance the active step, store its value,
/// and emit the next agent prompt + quick replies.
fn handle_onboard(w: &Weak<StudioWindow>, st: &Rc<RefCell<RunState>>, input: &str) {
    {
        let mut s = st.borrow_mut();
        s.ob_msgs.push(umsg(input));
        let i = match s.ob_steps.iter().position(|x| !x.done) {
            Some(i) => i,
            None => return,
        };
        let id = s.ob_steps[i].id.to_string();
        let t = input.to_lowercase();
        let value: String = match id.as_str() {
            "workspace" => {
                if t.contains("team") {
                    s.ob_team = true;
                    "Team workspace".into()
                } else {
                    "Personal workspace".into()
                }
            }
            "runtime" => if t.contains("ollama") { "Ollama :11434".into() }
                else if t.contains("llama") { "llama.cpp :8080".into() }
                else { "Embedded Gemma · GGUF".into() },
            "roster" => if t.contains("solo") { "Solo · Operator only".into() }
                else if t.contains("manual") { "4 signers · manual".into() }
                else { "5 signers · org directory".into() },
            "capsules" => if t.contains("basic") { "3 signed capsules".into() } else { "5 signed capsules".into() },
            "oversight" => if t.contains("monitor") || t.contains("on-loop") || t.contains("on)") { "Human-on-loop".into() }
                else if t.contains("auto") || t.contains("out") { "Human-out · Low/Med only".into() }
                else { "Human-in-loop".into() },
            "prompt" => {
                let v: String = input.trim().chars().take(28).collect();
                if input.trim().chars().count() > 28 { format!("“{v}…”") } else { format!("“{v}”") }
            }
            _ => "Configured".into(),
        };
        s.ob_steps[i].value = value.into();
        s.ob_steps[i].done = true;
        s.ob_steps[i].active = false;
        if i + 1 < s.ob_steps.len() {
            s.ob_steps[i + 1].active = true;
        }
        let team = s.ob_team;
        let (next_msg, next_quick): (&str, Vec<String>) = match id.as_str() {
            "workspace" => (
                if team { "A team workspace — you'll keep your own personal workspace rights regardless of what the team grants or revokes. Now, a model runtime: I found a few on your network." }
                else { "Personal workspace it is — you'll own every right in it. Now, a model runtime: I found a few on your network." },
                svec(&["Ollama · :11434", "llama.cpp · :8080", "Embedded Gemma (GGUF)"]),
            ),
            "runtime" => ("Connected — I verified the model's SHA-256 before binding it. Next, who approves consequential actions? I'll enroll an approval roster.",
                svec(&["Use my org directory", "Add signers manually", "Solo for now"])),
            "roster" => ("Roster enrolled. Separation-of-duties is enforced for you — Compliance and Security can't both sign one action, and the Auditor never approves. Next, capsules.",
                svec(&["Install the reconciliation set", "Just the basics"])),
            "capsules" => ("Installed and signature-verified — anything unsigned stays locked, so I can't stage what the runtime would refuse to run. Last setting: how closely do you want to watch? You can change this per-capsule later.",
                svec(&["Approve each (in-loop)", "Monitor (on-loop)", "Auto within policy"])),
            "oversight" => ("Set. High and Critical always ask regardless. That's everything — your harness is ready. Try it: tell me an outcome you want, in plain words.",
                svec(&["Reconcile last night's ledger across all facilities"])),
            "prompt" => ("Good — I can stage that now: snapshot, pull CUI, cross-match PHI behind a 2-of-N gate, write the report, anchor the root. Open your Studio and I'll show you the composition.",
                vec![]),
            _ => ("", vec![]),
        };
        s.ob_msgs.push(amsg(next_msg));
        s.ob_quick = next_quick;
        if id == "prompt" {
            s.ob_ready = true;
        }
    }
    if let Some(ui) = w.upgrade() {
        refresh(&ui, &st.borrow());
    }
}

/// Recompute the Audit Scrubber verdict through the seam (real
/// `AuditChain::verify_integrity` under `core-live`) and push it to the UI.
fn set_audit_verdict(ui: &StudioWindow, tampered: bool) {
    let frames: Vec<(u64, String, String)> = data::frames()
        .iter()
        .map(|f| (f.seq as u64, f.evt.to_string(), f.actor.to_string()))
        .collect();
    let v = audit_verify::verify(&frames, tampered);
    let app = ui.global::<AppState>();
    app.set_scrubber_verdict(v.message.into());
    app.set_scrubber_ok(v.ok);
    app.set_scrubber_break_seq(v.break_seq);
}

fn seed_catalog(ui: &StudioWindow) {
    let app = ui.global::<AppState>();
    app.set_tools_chain(vm(data::tools_chain()));
    app.set_tools_code(vm(data::tools_code()));
    app.set_capsules(vm(data::capsules()));
    app.set_frames(vm(data::frames()));
    app.set_doctor(vm(data::doctor()));
    app.set_tripwires(vm(data::tripwires()));
    app.set_signers(vm(data::signers()));
}

/// Apply a demo seed for screenshots / dev (mirrors shell.jsx SEED).
fn apply_seed(st: &Rc<RefCell<RunState>>, seed: &str) {
    let mut s = st.borrow_mut();
    match seed {
        "running" => s.status = "running".into(),
        "gate" => {
            s.playhead = 18.0;
            s.status = "paused".into();
            s.pending = Some("c3".into());
            s.medium = vec!["c2".into()];
            s.auto = vec!["recon.snapshot".into()];
            s.trip_fired = true;
        }
        "done" => {
            s.playhead = 56.0;
            s.status = "done".into();
            s.trip_fired = true;
            s.auto = vec!["recon.snapshot".into(), "recon.anchor-merkle".into()];
        }
        _ => {}
    }
}

fn main() -> Result<(), slint::PlatformError> {
    // ---- headless snapshot path (no window surface, software renderer) ----
    // CITRATE_STUDIO_SHOT=out.png [CITRATE_STUDIO_SEED=gate|done|running]
    // [CITRATE_STUDIO_SELECT=c3] [CITRATE_STUDIO_W=1360 CITRATE_STUDIO_H=880]
    if std::env::var("CITRATE_STUDIO_SHOT").is_ok() {
        return headless_shot();
    }

    let ui = StudioWindow::new()?;
    let app = ui.global::<AppState>();
    seed_catalog(&ui);
    let st = new_state();
    refresh(&ui, &st.borrow());
    set_audit_verdict(&ui, false);

    // ---- helper to refresh from a weak handle ----
    let refresh_weak = {
        let st = st.clone();
        move |w: &Weak<StudioWindow>| {
            if let Some(ui) = w.upgrade() {
                refresh(&ui, &st.borrow());
            }
        }
    };

    // ---- intents ----
    macro_rules! on {
        ($setter:ident, $body:expr) => {{
            let st = st.clone();
            let w = ui.as_weak();
            let rf = refresh_weak.clone();
            app.$setter(move |arg| {
                #[allow(clippy::redundant_closure_call)]
                ($body)(&st, &arg);
                rf(&w);
            });
        }};
    }
    macro_rules! on0 {
        ($setter:ident, $body:expr) => {{
            let st = st.clone();
            let w = ui.as_weak();
            let rf = refresh_weak.clone();
            app.$setter(move || {
                #[allow(clippy::redundant_closure_call)]
                ($body)(&st);
                rf(&w);
            });
        }};
    }

    on0!(on_play, |st: &Rc<RefCell<RunState>>| {
        let mut s = st.borrow_mut();
        if s.status == "done" {
            s.reset();
        }
        s.status = "running".into();
    });
    on0!(on_pause, |st: &Rc<RefCell<RunState>>| {
        st.borrow_mut().status = "paused".into();
    });
    on0!(on_rewind, |st: &Rc<RefCell<RunState>>| {
        st.borrow_mut().reset();
    });
    on0!(on_step, |st: &Rc<RefCell<RunState>>| {
        let mut s = st.borrow_mut();
        let ph = s.playhead;
        let next_end = s
            .base
            .iter()
            .map(|c| (c.start + c.dur) as f32)
            .filter(|e| *e > ph + 0.01)
            .fold(f32::INFINITY, f32::min);
        s.playhead = next_end.min(UNITS_TOTAL);
    });
    on0!(on_step_back, |st: &Rc<RefCell<RunState>>| {
        let mut s = st.borrow_mut();
        s.playhead = (s.playhead - 8.0).max(0.0);
    });
    on0!(on_resume, |st: &Rc<RefCell<RunState>>| {
        let mut s = st.borrow_mut();
        if let Some(id) = s.pending.take() {
            s.approved.push(id);
        }
        s.signatures.clear();
        s.status = "running".into();
    });
    on!(on_sign, |st: &Rc<RefCell<RunState>>, role: &SharedString| {
        let mut s = st.borrow_mut();
        let role = role.to_string();
        if let Some(sig) = s.signers.iter().find(|x| x.role == role) {
            let name = sig.name.to_string();
            s.signatures.push((role, name, "Slint".into()));
        }
    });
    on!(on_medium_approve, |st: &Rc<RefCell<RunState>>, id: &SharedString| {
        let mut s = st.borrow_mut();
        let id = id.to_string();
        s.medium.retain(|x| x != &id);
    });
    on!(on_toggle_dry, |st: &Rc<RefCell<RunState>>, id: &SharedString| {
        let mut s = st.borrow_mut();
        let id = id.to_string();
        if s.dry.contains(&id) {
            s.dry.retain(|x| x != &id);
        } else {
            s.dry.push(id);
        }
    });
    on!(on_toggle_solo, |st: &Rc<RefCell<RunState>>, id: &SharedString| {
        let mut s = st.borrow_mut();
        let id = id.to_string();
        s.solo = if s.solo.as_deref() == Some(id.as_str()) { None } else { Some(id) };
    });
    on!(on_set_oversight, |st: &Rc<RefCell<RunState>>, v: &SharedString| {
        st.borrow_mut().oversight = v.to_string();
    });

    // selection / code / workspace / depth (touch AppState directly)
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_select_clip(move |id| {
            if let Some(ui) = w.upgrade() {
                let app = ui.global::<AppState>();
                if id.is_empty() {
                    app.set_selected_id("".into());
                    app.set_has_selected(false);
                } else {
                    app.set_selected_id(id.clone());
                    if let Some(c) = st.borrow().clip(&id) {
                        app.set_selected_clip(c);
                        app.set_has_selected(true);
                    }
                }
                refresh(&ui, &st.borrow());
            }
        });
    }
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_select_by_name(move |name| {
            if let Some(ui) = w.upgrade() {
                let app = ui.global::<AppState>();
                if let Some(c) = st.borrow().clip_by_name(&name) {
                    app.set_selected_id(c.id.clone());
                    app.set_selected_clip(c);
                    app.set_has_selected(true);
                    refresh(&ui, &st.borrow());
                }
            }
        });
    }
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_open_code(move |name| {
            if let Some(ui) = w.upgrade() {
                let app = ui.global::<AppState>();
                if let Some(c) = st.borrow().clip_by_name(&name) {
                    let (manifest, wit, calldata) = gen_source(&c);
                    app.set_code_clip(name.clone());
                    app.set_code_clip_data(c);
                    app.set_code_manifest(manifest.into());
                    app.set_code_wit(wit.into());
                    app.set_code_calldata(calldata.into());
                    app.set_code_revalidate(false);
                    app.set_code_tab("manifest".into());
                    app.set_has_code(true);
                    refresh(&ui, &st.borrow());
                }
            }
        });
    }
    {
        let w = ui.as_weak();
        app.on_set_workspace(move |v| {
            if let Some(ui) = w.upgrade() {
                ui.global::<AppState>().set_workspace(v);
            }
        });
    }
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_set_search(move |v| {
            if let Some(ui) = w.upgrade() {
                ui.global::<AppState>().set_palette_search(v);
                refresh(&ui, &st.borrow());
            }
        });
    }
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_go_depth(move |lvl| {
            if let Some(ui) = w.upgrade() {
                let app = ui.global::<AppState>();
                match lvl.as_str() {
                    "L0" => {
                        app.set_l0_visible(true);
                        app.set_selected_id("".into());
                        app.set_has_selected(false);
                        app.set_code_clip("".into());
                        app.set_has_code(false);
                    }
                    "L1" => {
                        app.set_selected_id("".into());
                        app.set_has_selected(false);
                        app.set_code_clip("".into());
                        app.set_has_code(false);
                        app.set_health_open(false);
                    }
                    "L2" => {
                        let s = st.borrow();
                        if let Some(c) = s.clip("c3") {
                            app.set_selected_id(c.id.clone());
                            app.set_selected_clip(c);
                            app.set_has_selected(true);
                        }
                    }
                    "L3" => app.set_health_open(true),
                    "L4" => {
                        let s = st.borrow();
                        let name = if app.get_has_selected() {
                            app.get_selected_clip().name.to_string()
                        } else {
                            "recon.match-phi".to_string()
                        };
                        if let Some(c) = s.clip_by_name(&name) {
                            app.set_code_clip(name.into());
                            app.set_code_clip_data(c);
                            app.set_has_code(true);
                        }
                    }
                    _ => {}
                }
                refresh(&ui, &st.borrow());
            }
        });
    }

    app.on_ttl_fmt(|s| format!("{}:{:02}", s / 60, s % 60).into());

    // audit scrubber — recompute the real verify_integrity verdict on toggle
    {
        let w = ui.as_weak();
        app.on_toggle_tamper(move || {
            if let Some(ui) = w.upgrade() {
                let tampered = !ui.global::<AppState>().get_scrubber_tampered();
                ui.global::<AppState>().set_scrubber_tampered(tampered);
                set_audit_verdict(&ui, tampered);
            }
        });
    }

    // ---- chat ----
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_chat_send(move |text| handle_chat(&w, &st, &text));
    }
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_chat_chip(move |text| handle_chat(&w, &st, &text));
    }
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_chat_approve(move || {
            {
                let mut s = st.borrow_mut();
                s.pending = None;
                s.signatures.clear();
                s.chat.push(amsg("Quorum met — the core returned Approved. Writing the report and anchoring the Merkle root now."));
                s.chat.push(ChatMsg {
                    kind: "card".into(),
                    text: "".into(),
                    card_title: "Nightly reconciliation · complete".into(),
                    steps: vm(vec![
                        cstep("recon.snapshot", "low", "1,284 heads"),
                        cstep("recon.fetch-records", "medium", "3,402 CUI"),
                        cstep("recon.match-phi", "high", "7 anomalies"),
                        cstep("recon.write-report", "medium", "report written"),
                        cstep("recon.anchor-merkle", "low", "0x4c9f… anchored"),
                    ]),
                    cta: "Open the audit replay in Advanced".into(),
                });
                s.chat_quick = svec(&["What were the 7 anomalies?", "Run something else"]);
            }
            if let Some(ui) = w.upgrade() {
                refresh(&ui, &st.borrow());
            }
        });
    }
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_open_advanced_run(move || {
            if let Some(ui) = w.upgrade() {
                let app = ui.global::<AppState>();
                app.set_workspace("advanced".into());
                app.set_chat_open(false);
                {
                    let mut s = st.borrow_mut();
                    s.reset();
                    s.status = "running".into();
                }
                refresh(&ui, &st.borrow());
            }
        });
    }

    // ---- settings / rbac ----
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_rbac_drop(move |id| {
            {
                let mut s = st.borrow_mut();
                if let Some(m) = s.rbac.iter().find(|m| m.id == id) {
                    let name = m.name.to_string();
                    s.rbac_dropped.insert(0, name);
                    s.rbac_dropped.truncate(3);
                }
                s.rbac.retain(|m| m.id != id);
            }
            if let Some(ui) = w.upgrade() {
                refresh(&ui, &st.borrow());
            }
        });
    }
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_rbac_add(move |id| {
            {
                let mut s = st.borrow_mut();
                if let Some(p) = data::directory().into_iter().find(|p| p.id == id) {
                    let mut p = p;
                    p.team_role = "Member".into();
                    s.rbac.push(p);
                }
            }
            if let Some(ui) = w.upgrade() {
                ui.global::<AppState>().set_rbac_adding(false);
                refresh(&ui, &st.borrow());
            }
        });
    }

    // ---- onboarding ----
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_onboard_send(move |text| handle_onboard(&w, &st, &text));
    }
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_onboard_chip(move |text| handle_onboard(&w, &st, &text));
    }
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_enter_studio(move || {
            if let Some(ui) = w.upgrade() {
                let app = ui.global::<AppState>();
                app.set_view("studio".into());
                app.set_workspace("beginner".into());
                refresh(&ui, &st.borrow());
            }
        });
    }
    {
        let st = st.clone();
        let w = ui.as_weak();
        app.on_start_onboarding(move || {
            {
                let mut s = st.borrow_mut();
                s.ob_steps = data::onboard_steps();
                s.ob_msgs = vec![amsg("Welcome, Aleia. I'll have you prompting in about a minute — and you'll watch every choice land on the right. First: is this workspace for just you, or a team?")];
                s.ob_quick = svec(&["Just me", "A team"]);
                s.ob_ready = false;
                s.ob_team = false;
            }
            if let Some(ui) = w.upgrade() {
                ui.global::<AppState>().set_view("onboard".into());
                refresh(&ui, &st.borrow());
            }
        });
    }

    // ---- playback loop ----
    let play_timer = slint::Timer::default();
    {
        let st = st.clone();
        let w = ui.as_weak();
        play_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(TICK_MS),
            move || {
                {
                    let mut s = st.borrow_mut();
                    if s.status != "running" {
                        return;
                    }
                    s.tick();
                }
                if let Some(ui) = w.upgrade() {
                    refresh(&ui, &st.borrow());
                }
            },
        );
    }

    // ---- ttl countdown (human-out-of-loop) ----
    let ttl_timer = slint::Timer::default();
    {
        let st = st.clone();
        let w = ui.as_weak();
        ttl_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_secs(1),
            move || {
                let mut tick = false;
                {
                    let mut s = st.borrow_mut();
                    if s.oversight == "out" && s.ttl > 0 {
                        s.ttl -= 1;
                        tick = true;
                    }
                }
                if tick {
                    if let Some(ui) = w.upgrade() {
                        refresh(&ui, &st.borrow());
                    }
                }
            },
        );
    }

    // Dev smoke test: CITRATE_STUDIO_SMOKE=ms auto-quits the live run
    // (startup validation without hanging).
    if let Ok(ms) = std::env::var("CITRATE_STUDIO_SMOKE") {
        if let Ok(ms) = ms.parse::<u64>() {
            slint::Timer::single_shot(std::time::Duration::from_millis(ms), || {
                slint::quit_event_loop().ok();
            });
        }
    }

    ui.run()
}

/// Render the UI headlessly with the software renderer into a buffer and
/// save it as PNG — works without a window surface (this shell has none).
fn headless_shot() -> Result<(), slint::PlatformError> {
    use slint::platform::software_renderer::{
        MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
    };
    use slint::platform::{Platform, WindowAdapter};
    use std::rc::Rc;

    struct HeadlessPlatform {
        window: Rc<MinimalSoftwareWindow>,
    }
    impl Platform for HeadlessPlatform {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
            Ok(self.window.clone())
        }
    }

    let w: u32 = std::env::var("CITRATE_STUDIO_W").ok().and_then(|v| v.parse().ok()).unwrap_or(1360);
    let h: u32 = std::env::var("CITRATE_STUDIO_H").ok().and_then(|v| v.parse().ok()).unwrap_or(880);
    let path = std::env::var("CITRATE_STUDIO_SHOT").unwrap();

    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(HeadlessPlatform { window: window.clone() }))
        .map_err(|e| slint::PlatformError::Other(format!("set_platform: {e}")))?;

    let ui = StudioWindow::new()?;
    window.set_size(slint::PhysicalSize::new(w, h));
    seed_catalog(&ui);
    let st = new_state();
    if let Ok(seed) = std::env::var("CITRATE_STUDIO_SEED") {
        apply_seed(&st, &seed);
    }
    if let Ok(sel) = std::env::var("CITRATE_STUDIO_SELECT") {
        if let Some(c) = st.borrow().clip(&sel) {
            let app = ui.global::<AppState>();
            app.set_selected_id(c.id.clone());
            app.set_selected_clip(c);
            app.set_has_selected(true);
        }
    }
    if let Ok(view) = std::env::var("CITRATE_STUDIO_VIEW") {
        ui.global::<AppState>().set_view(view.into());
    }
    if let Ok(ws) = std::env::var("CITRATE_STUDIO_WS") {
        ui.global::<AppState>().set_workspace(ws.into());
    }
    if let Ok(ov) = std::env::var("CITRATE_STUDIO_OVERLAY") {
        let app = ui.global::<AppState>();
        match ov.as_str() {
            "settings" => app.set_settings_open(true),
            "chat" => app.set_chat_open(true),
            "health" => app.set_health_open(true),
            "breakglass" => app.set_breakglass_open(true),
            "code" => {
                if let Some(c) = st.borrow().clip_by_name("recon.match-phi") {
                    let (m, wit, cd) = gen_source(&c);
                    app.set_code_clip_data(c);
                    app.set_code_manifest(m.into());
                    app.set_code_wit(wit.into());
                    app.set_code_calldata(cd.into());
                    app.set_has_code(true);
                }
            }
            _ => {}
        }
    }
    refresh(&ui, &st.borrow());
    set_audit_verdict(&ui, std::env::var("CITRATE_STUDIO_TAMPER").is_ok());

    // Render. Two passes so layout settles before the captured frame.
    let mut buf = vec![PremultipliedRgbaColor::default(); (w * h) as usize];
    slint::platform::update_timers_and_animations();
    window.draw_if_needed(|renderer| { renderer.render(&mut buf, w as usize); });
    window.request_redraw();
    window.draw_if_needed(|renderer| { renderer.render(&mut buf, w as usize); });

    // Un-premultiply to straight RGBA8 for PNG.
    let mut rgba = Vec::with_capacity(buf.len() * 4);
    for p in &buf {
        let a = p.alpha;
        let unmul = |c: u8| if a == 0 { 0 } else { ((c as u32 * 255) / a as u32) as u8 };
        rgba.push(unmul(p.red));
        rgba.push(unmul(p.green));
        rgba.push(unmul(p.blue));
        rgba.push(a);
    }
    let file = std::fs::File::create(&path).map_err(|e| slint::PlatformError::Other(format!("{e}")))?;
    let bw = std::io::BufWriter::new(file);
    let mut enc = png::Encoder::new(bw, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| slint::PlatformError::Other(format!("{e}")))?;
    wr.write_image_data(&rgba).map_err(|e| slint::PlatformError::Other(format!("{e}")))?;
    eprintln!("headless snapshot saved: {path} ({w}x{h})");
    Ok(())
}


// =============================================================
// Tests — STUDIO-1 run state machine + STUDIO-3 policy seam.
// Backfill (2026-06-04): repays STUDIO-1's acknowledged test debt and
// covers STUDIO-3's policy. The `policy::*` tests guard the parity
// between the default hand-rolled rules and the real core (core-live):
// both feature builds must pass these identically.
// =============================================================
#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> std::rc::Rc<std::cell::RefCell<RunState>> {
        new_state()
    }
    fn sig(role: &str) -> (String, String, String) {
        (role.into(), "Test Signer".into(), "Slint".into())
    }

    #[test]
    fn tick_pauses_at_high_gate_with_side_effects() {
        let st = state();
        let mut s = st.borrow_mut();
        s.status = "running".into();
        for _ in 0..400 {
            if s.status != "running" {
                break;
            }
            s.tick();
        }
        // c1 (auto 0–8) completed, c2 (medium @8) entered, c3 (high @18) gates.
        assert_eq!(s.status, "paused");
        assert_eq!(s.pending.as_deref(), Some("c3"));
        assert_eq!(s.playhead, 18.0, "pauses exactly at the gate start");
        assert!(s.medium.iter().any(|x| x == "c2"), "medium card queued");
        assert!(s.auto.iter().any(|x| x == "recon.snapshot"), "auto-approved feed");
        assert!(!s.trip_fired, "tripwire (>=22) not reached before the gate");
        assert!(s.signatures.is_empty());
    }

    #[test]
    fn tick_runs_to_completion_when_gate_pre_approved() {
        let st = state();
        let mut s = st.borrow_mut();
        s.approved.push("c3".into()); // skip the high gate
        s.status = "running".into();
        for _ in 0..400 {
            if s.status != "running" {
                break;
            }
            s.tick();
        }
        assert_eq!(s.status, "done");
        assert_eq!(s.playhead, UNITS_TOTAL);
        assert!(s.trip_fired, "TRIP-AU-002 fires past unit 22");
        assert!(s.auto.iter().any(|x| x == "recon.snapshot"));
        assert!(s.auto.iter().any(|x| x == "recon.anchor-merkle"));
    }

    #[test]
    fn quorum_met_needs_two_signatures_for_high() {
        let st = state();
        let mut s = st.borrow_mut();
        s.pending = Some("c3".into());
        assert!(!s.quorum_met(), "0/2");
        s.signatures.push(sig("Reviewer"));
        assert!(!s.quorum_met(), "1/2");
        s.signatures.push(sig("ComplianceOfficer"));
        assert!(s.quorum_met(), "2/2");
    }

    #[test]
    fn approval_roster_enforces_separation_of_duties() {
        let st = state();
        let mut s = st.borrow_mut();
        s.pending = Some("c3".into());
        s.signatures.push(sig("SecurityOfficer"));
        let rows = approval_roster(&s);
        let row = |role: &str| rows.iter().find(|r| r.role == role).cloned().unwrap();

        assert!(row("SecurityOfficer").signed);
        let co = row("ComplianceOfficer");
        assert!(co.disabled, "CO disabled once SO has signed");
        assert!(co.reason.starts_with("SoD"), "reason: {}", co.reason);
        let rv = row("Reviewer");
        assert!(!rv.signed && !rv.disabled, "Reviewer still free to sign");
    }

    #[test]
    fn quorum_satisfaction_matches_the_runtime() {
        // Passes under default (modeled) and core-live (Quorum::satisfied_by).
        let high = ["Reviewer".to_string(), "ComplianceOfficer".to_string(), "SecurityOfficer".to_string()];
        // High = NofM{2}: two from the required set satisfy; one does not.
        assert!(policy::quorum_satisfied("high", &high, &["Reviewer".into(), "ComplianceOfficer".into()]));
        assert!(!policy::quorum_satisfied("high", &high, &["Reviewer".into()]));
        // Auditor never counts toward a quorum.
        assert!(!policy::quorum_satisfied("high", &high, &["Auditor".into(), "Auditor".into()]));
        // Medium = OneOf: a single approver suffices.
        assert!(policy::quorum_satisfied("medium", &["Reviewer".into()], &["Reviewer".into()]));
        // Critical = Multiset{SO, CO, Reviewer}: all three required, regardless of manifest.
        let all3 = ["SecurityOfficer".to_string(), "ComplianceOfficer".to_string(), "Reviewer".to_string()];
        assert!(policy::quorum_satisfied("critical", &[], &all3));
        assert!(!policy::quorum_satisfied("critical", &[], &["SecurityOfficer".into(), "Reviewer".into()]));
    }

    #[test]
    fn policy_rules_match_the_runtime() {
        // SoD: CO ⊥ SO (symmetric); nothing else conflicts.
        assert!(policy::is_conflict("ComplianceOfficer", "SecurityOfficer"));
        assert!(policy::is_conflict("SecurityOfficer", "ComplianceOfficer"));
        assert!(!policy::is_conflict("Reviewer", "ComplianceOfficer"));
        // Auditor never approves; everyone else may.
        assert!(!policy::can_approve("Auditor"));
        assert!(policy::can_approve("Reviewer"));
        assert!(policy::can_approve("SecurityOfficer"));
        // Quorum::for_tier counts.
        assert_eq!(policy::quorum_n("low", &[]), 0);
        assert_eq!(policy::quorum_n("medium", &[]), 1);
        assert_eq!(policy::quorum_n("high", &[]), 2);
        assert_eq!(policy::quorum_n("critical", &[]), 3);
    }

    #[test]
    fn audit_verify_clean_and_tampered() {
        // Same assertions pass under the default (modeled) and `core-live`
        // (real AuditChain::verify_integrity) builds — the parity guard.
        let frames: Vec<(u64, String, String)> = data::frames()
            .iter()
            .map(|f| (f.seq as u64, f.evt.to_string(), f.actor.to_string()))
            .collect();
        assert_eq!(frames.len(), 12);

        let ok = audit_verify::verify(&frames, false);
        assert!(ok.ok, "clean chain verifies");
        assert!(ok.message.contains("12 frames"), "12 verified frames");
        assert_eq!(ok.break_seq, -1);
        assert!(ok.message.contains("ok"));

        let bad = audit_verify::verify(&frames, true);
        assert!(!bad.ok, "tamper detected");
        assert_eq!(bad.break_seq, 6, "broken link at sequence 6");
        assert!(bad.message.contains("sequence 6"), "message: {}", bad.message);
    }

    #[test]
    fn data_catalog_sanity() {
        let clips = data::clips();
        assert_eq!(clips.len(), 5);
        let c3 = clips.iter().find(|c| c.id == "c3").unwrap();
        assert_eq!(c3.risk, "high");
        assert_eq!(c3.quorum_n, 2);
        assert_eq!(c3.quorum_roles.iter().count(), 3);
        assert_eq!(data::tools_chain().len(), 11);
        assert_eq!(data::tools_code().len(), 6);
        assert_eq!(data::doctor().len(), 11);
        assert_eq!(data::tripwires().len(), 9);
        assert_eq!(data::signers().len(), 5);
    }
}
