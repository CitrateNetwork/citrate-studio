# Citrate Studio — Design Specification for Claude Design

**A build-ready interface spec for the native + web app that drives the Citrate agent runtime.**

Author: handoff from runtime engineering • Target: Claude Design (prototype build on the Citrate design system)
Source of truth: `citrate-agent-runtime` @ branch `audit/rm-g2-breakglass-sig-2026-06-01`
Companion: this document refines the *Citrate Studio interface vision* into the literal primitives the runtime enforces. Where the vision and the code disagree, **the code wins** and is annotated below.

---

## 0. How to read this document

This spec is organized so a designer can build screen-by-screen without reading Rust. Every UI element is traced to a **real runtime primitive** — a struct, enum, function, or constant that already exists (or is explicitly marked as a stub). The format is:

> **What the user sees** → **the primitive it renders** (`file:line`) → **the exact values/states it can take**.

Three rules govern the whole product and recur throughout:

1. **Depth, not sprawl.** Complexity descends a z-axis (L0→L4), it never spreads sideways. Only L0 and L1 are ambient; L2–L4 are reached by intent.
2. **The front end never makes a policy decision.** It is a *viewport* and an *intent submitter*. Every gate (approval, quorum, allow-list, capability, integrity) stays behind the Rust core. The UI renders state and submits signed intents; it never grants itself permission.
3. **Two zones, always visually distinct.** The *editable composition* above (capsules, policy, config — non-destructive, reversible) and the *sealed ledger* below (the append-only, hash-chained audit record — permanent, destructive-of-deniability). These must never look like the same surface.

A critical honesty note up front (see also §11, "Implementation reality"): the **trust spine is fully built** — capsule manifest/verification, HITL approval/quorum/roster, break-glass, audit chain, doctor, tripwires, the cron/SOP engine, and the chain/code tool inventory all exist in code today. The **agent loop and model resolver are currently stubs** (`agent/core/src/agent/mod.rs`, `agent/core/src/model/mod.rs` — ~10 lines each, deferred to sprint CIT-AGENT-3). So **L0 (Outcome/chat)** is the one layer the runtime cannot yet fully back. Design it, but treat its live data as forthcoming; build the demo and first gates on L1–L4, which are real today. This is also why the recommended first ship (§10, Gate 1) is the read-only Audit Scrubber: it rides entirely on shipped code.

---

## 1. The depth model, bound to real primitives

Five layers. A user sits at L0 forever and gets work done; a compliance officer lives at L3; an engineer drops to L4. The descent rule — **every layer down removes one abstraction and adds one truth** — is literal here, because each layer surfaces a primitive that is strictly closer to the bytes.

| Layer | Name | What it shows | Backing primitive (file) | Status |
|---|---|---|---|---|
| **L0** | Outcome | Chat + outcome cards; the swarm "just works" | agent loop, model resolver (`agent/mod.rs`, `model/mod.rs`) | ⚠️ **stub** — design now, wire later |
| **L1** | Composition | A run as a layer-stack on a timeline; watch it execute | `CapsuleDispatch` (`capsule/dispatch.rs`), SOP engine (`agent-cron/src/sop.rs`), cron (`agent-cron/src/scheduler.rs`) | ✅ real |
| **L2** | Inspector | One capsule's declared capabilities, risk, data classes, allow-list | `Manifest` (`capsule/manifest.rs`), `RiskTier`, `CapabilitySet`, `DataClassDecl` | ✅ real |
| **L3** | Trust | Approvals, quorum, roster, break-glass, doctor, tripwires | `ApprovalQueue` (`hitl/mod.rs`), `Quorum` (`hitl/quorum.rs`), `SignerRoster` (`hitl/signing.rs`), break-glass (`hitl/break_glass.rs`), `Doctor` (`doctor/`), tripwires (`agent-cron/src/tripwires/`) | ✅ real |
| **L4** | Code | The literal manifest, WIT, calldata bytes, WASM | `capsule.wit`, `archive.rs` (`.cps`), `linker.rs`, `tiers.rs` | ✅ real (read; edit-with-revalidate is the build target) |

**Progressive-disclosure budget:** only L0 and L1 are ambient (always reachable in the chrome). L2 is one click *into a layer*. L3 is one click *into a property* (or via the persistent Health Strip). L4 is the deepest twirl. Never push a user past the layer they asked for.

---

## 2. The trust boundary (the contract the UI must honor)

This is the single most important architectural constraint for the designer, because it determines what every button is *allowed* to do.

```
┌─────────────────────────────────────────────────────────────┐
│  CITRATE STUDIO FRONT END  (Slint native  |  Web/WASM)      │
│  • renders state           • submits intents                │
│  • NEVER decides: approval / quorum / allow-list / capability│
└───────────────┬─────────────────────────────────────────────┘
                │  intents (typed)            state (subscriptions)
                ▼                              ▲
┌─────────────────────────────────────────────────────────────┐
│  citrate-agent-core   (rlib + cdylib — agent/core/Cargo.toml)│
│  ApprovalQueue · Quorum · SignerRoster · CapsuleDispatch ·   │
│  AuditChain · Doctor · break-glass · tripwires · grants      │
│  Every gate lives here. Re-exported via lib.rs (frozen API). │
└───────────────┬─────────────────────────────────────────────┘
                │ anchors / reads
                ▼
        citrate-chain  (L1 BlockDAG, chain_id 40204, AnchorRegistry + tripwire/registry contracts)
```

**The core is built as both `rlib` and `cdylib`** (`agent/core/Cargo.toml: crate-type = ["rlib","cdylib"]`), which is exactly why one trust core can back two shells. The frozen public surface the UI may call is small and named in `lib.rs`: `ApprovalQueue`, `RecorderClient`, `ApprovalOutcomePublic`, `PendingView`, `ToolCall`, `ToolResult`, plus `CapsuleDispatch::call_raw`.

**What "submit an intent" means concretely.** The UI may:
- *render* a pending approval (`ApprovalQueue::signatures_on(call_id)`, `payload_for(call_id)`),
- *collect a signature* on a signing surface and hand back an `AttestedSignature` over the exact `payload` bytes,
- *call* `add_signature(call_id, sig)` — and the **core** decides whether quorum is met.

The UI must never compute "approved." It displays what the core returns (`ApprovalOutcomePublic::{AutoApproved, Approved, Rejected, …}`).

**Signing-surface identity is recorded.** Every signature carries a `SigningSurfaceTag` (`audit/record.rs:51`): `Slint | Cli | LocalWeb | Mobile | FileBacked`. The native shell stamps `Slint`; the web shell stamps `LocalWeb`. High/critical signatures in the web shell must route to attested hardware (a browser cannot safely hold a SecurityOfficer key) — design the hand-off, don't design a browser key vault.

---

## 3. Shared visual language (use the existing Citrate design system)

Do not invent tokens. The Citrate system is **warm-evergreen, paper-first, dark-capable**, with a single restrained green accent and a hard rule that hashes/data are de-emphasized. Pull exact values from `citrate-explorer/src/scan/scan.css` and `citrate-buyer-webapp/tailwind.config.ts`. Component foundation is **shadcn/ui** on Tailwind.

### 3.1 Color

| Token | Hex | Use in Studio |
|---|---|---|
| Citrate Green (primary) | `#8ecc09` | Accent, primary CTAs, "executing" playhead, ✦ on-chain marks |
| Green Deep (AA text) | `#5a8205` | Text/buttons on paper |
| Green Dark | `#2f4502` | Pressed |
| Citrate Yellow (secondary) | `#ffbd10` | Evidence markers, **Warn** doctor state, tripwire amber |
| Danger | `#a72414` (light) / `#e26a58` (dark) | **Blocker**, broken audit link, rejected approval, red blocks |
| Evergreen header | `#0f2a1a` (light) / `#081a10` (dark) | The **sealed-ledger zone** chrome (see §3.5) |
| Paper canvas | `#f1eee6` (light) / `#0c2216` (dark) | The **editable-composition zone** |
| Text-3 (muted) | `#8a8c84` / `#7a9a84` | All hashes, addresses, byte data — always |

**Risk-tier color system (define once, reuse everywhere).** This is the product's most repeated signal. Bind it to `RiskTier` (`capsule/manifest.rs:141`):

| `RiskTier` | toml | Color intent | Approval behavior it implies |
|---|---|---|---|
| `Low` | `low` | Green `#8ecc09` / calm | Auto-approve, silent — no card |
| `Medium` | `medium` | Yellow `#ffbd10` | One ambient, dismissable card |
| `High` | `high` | Orange/gold `#b07b00` | Quorum card (2-of-N) |
| `Critical` | `critical` | Danger `#a72414` | Full-stop modal (SO+CO+Reviewer) |

### 3.2 Type
- **Space Grotesk** — display/titles (layer names, run titles). Tracking `-0.022em`.
- **Geist** — UI body, labels, nav.
- **Geist Mono** — *every* hash, address, selector, byte offset, DID, content-hash. Smaller, `text-3` muted. This is a system rule, not a preference.
- **Cormorant** — editorial/narrative (onboarding, empty-state prose) only.
- Eyebrow labels: caps, `0.14em`.

### 3.3 Motion (the run must feel like motion design, not a log)
Use the existing tokens: `fast 140ms`, `base 220ms`, `slow 420ms` on `cubic-bezier(.2,0,0,1)`; `spring cubic-bezier(.34,1.18,.4,1)` for cards/clips blooming in; `narrative 900ms` for streaming agent text. **Respect `prefers-reduced-motion`** — the playhead jumps instead of sweeping; cards place instantly.

### 3.4 Depth / shadow
`hairline → 1 → 2 → lift → pop`, plus the green focus ring `0 0 0 3px rgba(142,204,9,.35)`. Use elevation to signal *which layer you're in*: L0/L1 flat on canvas; L2 Inspector lifts (`lift`); L3 docks `pop`; L4 Code Drawer is a full modal floor.

### 3.5 The two-zone rule (non-negotiable)
The screen has an **editable zone** (composition, policy adjustment layers, capsule palette, trust config) rendered on **paper canvas**, and a **sealed ledger zone** (audit chain, anchored records, firing records) rendered on **evergreen** with a subtly different texture/chrome. The contrast itself communicates the core inversion: *config above is reversible; the record below is permanent.* The Audit Scrubber and the on-chain firing records live in the evergreen zone; the Composition Canvas, Palette, Inspector, and Trust Panel live on paper.

---

## 4. Master data dictionary (render these exactly)

Every enum the UI displays, with its exact variants and serialized string. Treat these as the only legal values; do not localize the serialized strings.

**`RiskTier`** (`manifest.rs:141`) → `low`, `medium`, `high`, `critical`.

**`Role`** (`manifest.rs:150`) → `Operator`, `Reviewer`, `ComplianceOfficer`, `SecurityOfficer`, `Auditor`.

**`NetworkPolicy`** (`manifest.rs:95`) → `none`, `broker-only`, `egress-allowed`.

**`DataClass`** (`manifest.rs:114`) → `PUBLIC`, `CUI`, `PHI`, `FERPA`, `FERPA-directory`, `FERPA-restricted`, `ITAR`. (ITAR is the one that hard-blocks break-glass — give it a distinct, heavier treatment.)

**`SigningTier`** (`manifest.rs:188`) → `bundled` (Citrate canonical FIPS-HSM key), `managed` (org procurement CA), `workspace` (operator local hardware key).

**`CapabilityToken`** (linker, `capsule/linker.rs:36`) — the capability *glyph row* on every capsule tile/inspector: `WasiCli`, `WasiClocks`, `WasiRandom`, `WasiSockets` (network), `WasiFilesystem` (fs), `CitrateChainEthCall` (chain-read), `CitrateChainEthSend` (chain-write). Design one glyph per token.

**`Quorum`** (`hitl/quorum.rs:27`) → `AutoApprove` | `OneOf(roles)` | `NofM { n, m }` (default n=2) | `Multiset(roles)`. The tier→quorum mapping is computed by `Quorum::for_tier` (`quorum.rs:48`): Low→AutoApprove, Medium→OneOf(manifest roles), High→NofM{2, manifest roles}, **Critical→Multiset{SecurityOfficer, ComplianceOfficer, Reviewer} regardless of manifest**.

**Separation-of-duties** (`hitl/roles.rs`): `is_conflict` is true only for the **(ComplianceOfficer, SecurityOfficer)** pair — they may not both sign one action. `can_approve(Auditor) == false` — **Auditor never approves** (read-only). Plus: no self-approval (proposer can't sign), no duplicate signer (one pubkey, one signature). The UI must *enforce these before submission* (gray out illegal signer choices) — but the core re-checks and is the authority.

**`BreakGlassPhase`** (`hitl/break_glass.rs:53`) → `Pending → Invoked → {Affirmed | Unaffirmed} → Surfaced`. `AFFIRMATION_WINDOW = 72h` (`break_glass.rs:31`). Affirmation quorum = `{Reviewer, ComplianceOfficer}` (`break_glass.rs:44`). ITAR-touching manifests are hard-blocked from break-glass (`manifest_touches_itar`, `break_glass.rs:339`).

**`EventType`** (audit, `audit/record.rs:24`) — 19 variants, each a frame type in the scrubber: `Genesis, Proposal, Edit, Approval, Rejection, Timeout, Submission, Confirmation, CapsuleInstall, CapsuleRevoke, PolicyUpdate, OverlayActivation, OverlayDeactivation, BreakGlass, BreakGlassAffirmation, DoctorReport, AuditExport, Quarantine, Resumption`. Give each an icon + color.

**`Severity`** (doctor, `doctor/report.rs:20`) → `Pass` (green), `Warn` (yellow), `Blocker` (red). Drives the Health Strip color.

**`SigningSurfaceTag`** (`audit/record.rs:51`) → `Slint, Cli, LocalWeb, Mobile, FileBacked`. Small provenance badge on every signature row.

**`AnchorStrategy` / `AnchorKind`** (`audit/sink/chain.rs:30`, `chain/anchor.rs:42`) → `PerCapsule(0)`, `PerApproval(1)`, `NightlyMerkle(2)`. Shown in the ledger zone as the anchoring cadence.

---

## 5. The Adobe mapping, made literal

| AE/PS/AI concept | Studio surface | Real primitive |
|---|---|---|
| **Composition** | one agent run (or a saved agent definition) | a run = ordered `CapsuleDispatch` sequence + its `AuditChain` |
| **Layer stack (bottom renders first)** | capsule call sequence (bottom executes first) | `CapsuleDispatch::load_from_dir` loads **alphabetically**; execution order is the stack order — make reorder explicit (see note) |
| **Playhead / CTI** | execution position | the live step pointer over the dispatch sequence |
| **Keyframes** | HITL gates | a keyframe = a `Quorum` gate; Critical = hard keyframe (full-stop) |
| **Scrubbing** | audit replay | drag = iterate `AuditChain` records (each `AuditRecord` is a frame); `verify_integrity()` proves the chain |
| **Nested composition** | SOP / sub-swarm | `SOPDefinition` collapses to one layer, opens to its own timeline |
| **Adjustment layers (PS)** | policy / risk / overlay config | `OverlayDecl`, policy bundle, the per-capsule oversight dial — modify behavior without editing the capsule |
| **Layer masks (PS)** | scope | mask a policy to a tenant / `DataClass` / `RiskTier` |
| **Symbols (AI)** | Capsule Palette | a `.cps` archive instance of a signed, versioned master (`archive.rs`) |

> **Design note on reorder = execution order.** The vision says "reorder by drag reorders execution." Today `CapsuleDispatch` resolves capsules by name and the *run* defines the call sequence; the loader's alphabetical scan is just discovery, not run order. So: drag-to-reorder is a legitimate L1 affordance, but it edits the **run's** ordered call list (an intent the core accepts), not the on-disk capsule set. Treat the layer stack as the editable run plan; "solo"/"eye-toggle" map to dry-run flags on the plan, not to mutating capsules.

---

## 6. Module specifications (the ten modules)

Each module below gives: **layer · purpose · the primitive it renders (with fields) · states · interactions/intents · empty/loading/error · design notes.** Build them as composable shadcn-based panels inside one app shell.

### 6.1 Composition Canvas — *L1, the heart*

**Purpose.** The layer stack on a timeline. Capsules are horizontal clips; a playhead sweeps left→right; output cards bloom above each clip as it returns. This is where "fun" lives — tactile drag, playhead sweep, cards animating in. **One spatial axis only:** top→bottom = execution order. Complexity descends into the Inspector, never sideways.

**Renders.**
- Each **clip** = one entry in the run's capsule call sequence. Clip header shows: capsule `name` + `version` (`CapsuleMetadata`, `manifest.rs:78`), risk-tier color band (`RiskTier`), and a capability glyph row (`CapabilityToken` set derived from `CapabilitySet`).
- **Keyframe diamonds** on a clip = its `Quorum` gate (from `Quorum::for_tier`). Low = no diamond; Medium = hollow; High = half-filled (shows `n/m`); Critical = solid red (full-stop).
- **Output card** above a clip = `ToolResult { success, output, data, error }` (`agent-legacy/src/tool.rs:38`). Success → green check + plain-English `output`; error → danger + `error`. `data` (structured JSON) is a twirl-down inside the card.
- **Playhead** = current step. As each capsule fires, its clip lights green (`#8ecc09`).

**Interactions / intents.**
- Drag a clip → reorder the **run plan** (intent: reorder call sequence).
- Eye-toggle a clip → exclude from a **dry run** (no capsule mutated).
- Solo a clip → run it alone.
- Click a clip → open **Inspector (L2)**.
- A keyframe that's "armed" routes to the **Approval Dock (L3)** when the playhead reaches it; the run pauses (this is a real `ApprovalQueue::submit_for_action` pause, not cosmetic).

**States.** Idle (plan, not yet run) · Running (playhead sweeping, clips lighting) · Paused-at-gate (a keyframe is awaiting quorum — clip pulses, dock opens) · Done (all cards landed) · Failed (a clip shows danger, run halts; tie to `EmergencyStop` if triggered).

**Empty/loading/error.** Empty = "Drag a capsule from the Palette to begin" (Cormorant prose, calm). Loading = clips skeleton. Error = the failed clip is danger-bordered; the rest dim.

**Design notes.** This must read as After Effects, not a log viewer. The differentiator versus a node graph is the *absence* of a second axis — protect it. Live output cards use `spring` entry; playhead uses `base`/`narrative` sweep; reduced-motion jumps the playhead.

---

### 6.2 Capsule Palette — *the tool bin*

**Purpose.** A searchable library of installed, signed capsules. Drag a tile to add it as a layer. This is also the "updated Scratch" surface: snap a tool together and the manifest writes itself underneath (opens the Code Drawer).

**Renders.** One tile per installed capsule. Tile shows:
- `name`, `version` (`CapsuleMetadata`).
- Risk-tier dot (color-coded `RiskTier`).
- Capability glyph row: network (`WasiSockets`), filesystem (`WasiFilesystem`), chain-read (`CitrateChainEthCall`), chain-write (`CitrateChainEthSend`) — derived from `CapabilitySet` + the linker token set.
- Signing-tier chip (`bundled`/`managed`/`workspace`).

**The 17 native tools as palette content.** Beyond user-authored `.cps` capsules, the runtime ships a tool inventory the palette should surface (these are the agent's hands). Each has a name, risk level, and read/write nature:

*Chain tools (`agent-chain/src/tools/`):*
| Tool | Risk | Nature |
|---|---|---|
| `check_balance` | Low | chain-read |
| `query_contract` | Low | chain-read (eth_call) |
| `list_models` | Low | read |
| `run_inference` | Low | read (local GGUF or on-chain) |
| `explain_tx` | Low | chain-read |
| `get_block_height` | Low | chain-read |
| `get_peer_count` | Low | chain-read |
| `get_recent_blocks` | Low | chain-read |
| `get_tx_history` | Low | chain-read |
| `send_tx` | **High** | **chain-write** (approval required) |
| `deploy_contract` | **High** | **chain-write** (approval required) |

*Code tools (`agent-code/src/tools/`):*
| Tool | Risk | Nature |
|---|---|---|
| `file_read` | Low | fs-read (workspace-confined) |
| `search_code` | Low | fs-read |
| `file_write` | Medium | fs-write (denies secrets: `.ssh/.aws/.env/*.pem/*.key`) |
| `file_edit` | Medium | fs-write (exact-match) |
| `git_ops` | Medium (commit/push) / Low (status/diff) | execute |
| `shell_exec` | **Critical** | execute (allowlisted binaries only; shell metacharacters rejected) |

**Fail-closed dimming (a real gate, surfaced).** Unsigned or content-hash-mismatched capsules render **dimmed with a lock glyph**. This mirrors the runtime: `capsule_body_verified` (`dispatch.rs:98`) rejects a capsule whose wasm isn't bound to a valid `content_hash`, unless `CITRATE_ALLOW_UNVERIFIED_CAPSULES=1` (dev-only). The UI **refuses to let you stage what the runtime would refuse to run.** A placeholder hash (`sha256:000…0`) = unverified = locked tile with a "pack & sign to enable" affordance.

**Interactions / intents.** Search/filter (by risk, capability, data class, certified overlay). Drag tile → add layer to the active Composition. "Build a capsule" → opens **Code Drawer (L4)** pre-filled with manifest skeleton + WIT stub + procedure template, capabilities declared by toggling glyphs (toggling a glyph writes the corresponding `CapabilitySet` field + emits the `CapabilityToken` the WIT must match).

**Empty/error.** Empty = "No capsules installed." Locked tiles always visible (don't hide them) so the user understands the trust boundary.

---

### 6.3 Inspector — *L2, progressive disclosure into a layer*

**Purpose.** Click any layer → the capsule's declared facts in AE twirl-down idiom. Each twirl is one step closer to code; the deepest links to L4.

**Renders (the manifest, twirled).** Bind directly to `Manifest` (`capsule/manifest.rs:33`), 8 sections:
- **Summary band (always visible):** `name`, `version`, risk-tier badge, one-line purpose (from procedure.md / capsule metadata).
- **Capabilities (twirl 1):** `CapabilitySet` (`manifest.rs:85`) — `network: NetworkPolicy`; `filesystem: Vec<String>` (format `read|write|both:/path`); `chain_calls: Vec<String>` (format `eth_call|eth_send:0x…`) **rendered as the exact contract addresses it may touch**; `subagent_spawn: bool` (v1 always false — show as locked).
- **Data classes (twirl 2):** `DataClassDecl` (`manifest.rs:103`) — three rows: `reads`, `writes`, `emits`, each a set of `DataClass` chips. ITAR chip is visually heavier.
- **Risk (twirl):** `RiskDecl` (`manifest.rs:133`) — `tier`, `required_roles: Vec<Role>`, `break_glass_eligible: bool`.
- **Overlay (twirl):** `OverlayDecl` (`manifest.rs:159`) — `certified: Vec<String>` (e.g. `FedRAMP-High, CMMC-L3, ITAR`), `not_certified`.
- **Procedure (twirl):** `ProcedureDecl.gates: Vec<GateStep{ step, role }>` (`manifest.rs:167`) — the human steps and which `Role` must approve each.
- **Provenance (twirl, deepest):** `ProvenanceDecl` (`manifest.rs:179`) — `publisher` (DID, e.g. `did:citrate:agent:0xab12`), `build_reproducible: bool`, `agentile_sprint`, `tla_spec`; plus `content_hash` (mono, muted) and `SigningDecl.tier`. The "open in Code Drawer" link lives here.

**Interactions.** Twirl/untwirl (persist per-user defaults over time — beginner vs advanced workspace, Adobe-style). Click a contract address chip → "what is this" popover (resolve from `chain_calls` + known registries). Click content-hash or `tla_spec` → L4.

**Empty/error.** A capsule failing re-verification shows a Blocker banner mirroring the doctor's `capsule-manifest-reverify` check.

---

### 6.4 Approval Dock — *L3, HITL without fatigue*

**Purpose.** The module that most needs to be right: over-prompting causes blind-approval, which defeats oversight. The remedy is **risk-tier gating, which the runtime already implements** (`Quorum::for_tier`). The dock turns approvals from a nag into a trust instrument.

**Four presentations, driven by `RiskTier` → `Quorum`:**
- **Low → `AutoApprove`:** *no prompt.* Logged silently; visible only in the Audit Scrubber. (Most traffic. `submit_for_action` returns `AutoApproved` immediately, `hitl/mod.rs:366`.)
- **Medium → `OneOf(roles)`:** one ambient, inline-dismissable card. Single approver from the manifest's `required_roles`.
- **High → `NofM { n: 2, m }`:** a quorum card with **live signature progress** drawn from `signatures_on(call_id)` (`hitl/mod.rs:506`) — e.g. "Reviewer ✓ · ComplianceOfficer ⧗". Shows `n/m` filled.
- **Critical → `Multiset{SecurityOfficer, ComplianceOfficer, Reviewer}`:** a full-stop modal. The UI **enforces SoD before submission** (`hitl/roles.rs`): CO and SO can't both sign; Auditor can't appear; proposer can't self-approve; no duplicate pubkey. Illegal signer choices are disabled with an inline reason.

**Two features that make the dock a trust instrument:**

1. **The hash pin, made visible.** Every card states plainly: *"This approval covers exactly these bytes (`payload` hash). If the agent changes the request, the approval voids."* This is real: the queued action stores canonical `payload: Vec<u8>` (`RoleAwareEntry`, `hitl/mod.rs:211`); `add_signature` calls `verify_attestation(payload, sig)` and **rejects a signature that doesn't attest to those exact bytes** (`hitl/mod.rs:441`). `payload_for(call_id)` hands the signing surface the exact bytes to sign. Render the pinned hash (Geist Mono, muted) and a "what changed?" diff affordance if the payload ever differs.
2. **Three oversight modes as a per-capsule dial** — a Photoshop adjustment layer: **human-in-loop** (approve each), **human-on-loop** (monitor, intervene on anomaly), **human-out-of-loop** (auto within policy). Set globally, then *mask* a tighter mode onto sensitive capsules. This composes with the real `ApprovalFlow` auto-approve grant (`agent-legacy/src/approval.rs`): auto-approve is session-scoped and **time-bounded — `AUTO_APPROVE_DEFAULT_TTL_SECS = 3600`** (1h), and it *only ever bypasses Low/Medium; High/Critical always ask*. Show the dial's auto-approve state with a live countdown to expiry.

**States per card.** Pending (awaiting signatures; show roster slots) · Partially signed (progress) · Approved (`ApprovalOutcome::Approved`, resolver fires, run resumes) · Rejected (any signer rejects) · **Timed out** (`PENDING_TIMEOUT`; the role-aware queue expires) — show as a distinct neutral-fail, not an error.

**Empty.** "No approvals pending." Calm. A roster-health hint if no `SignerRoster` is configured (because `add_signature` fails closed without one — see §6.5).

**Design notes.** Ambient cards dock to a side rail; the Critical modal is the *one* place that interrupts. Pull approver identity + `SigningSurfaceTag` into each signature row.

---

### 6.5 Trust Panel — *L3, API & trust-layer config with secure defaults*

**Purpose.** Where every connection, key, RPC endpoint, and trust anchor is configured. Governing rule: **build from manifest, not filter default** — a new connector arrives with **zero capability** and the user opts *in*, never out.

**Secure-default presentation (loud default, quiet exception).** On any new API/connector:
- Network policy **`none`** until a reason is given (`NetworkPolicy::None`).
- Risk tier **`Critical`** until downgraded *with justification*.
- **Empty** chain-call allow-list; addresses added one at a time, each shown with *what it is*.
- Approval gate **required**; a write capability with no gate is **write-disabled** (mirrors `CodeAgentBridge`: the only public prod constructor is `with_approval`; `execute_tool` always calls `approval.check()` before running — `agent-code/src/bridge.rs:26,49`). Show "write-disabled — no gate" as an explicit, fixable state.

**Sub-panels.**
- **Signer Roster editor.** Enroll pubkeys per `Role` (`StaticSignerRoster::authorize(pubkey, role)`, `hitl/signing.rs:200`). Show each signer as `signer_id` (SHA-256 fingerprint of the 32-byte pubkey, `signing.rs:59`) + its role set. **Surface the fail-closed truth:** when no roster is configured, `add_signature` **fails closed in release builds** (`signer_roster: None` ⇒ rejects), so an unconfigured roster means *nothing high/critical can be approved*. Make that visible, not a silent dead-end.
- **Capability Grant view (the AGT-14 drift detector).** Show a grant's **registration-time snapshot** next to the **live grant**, so an operator sees at a glance whether anything drifted toward more permission. Bind to `CapabilityGrant` (`agent-legacy/src/canonical.rs:96`): `id, issuer, recipient, allowed_tools, max_value_per_tx, allowed_paths, expires_at, policy (ReadOnly|Guided|Operator|Maintainer), revoked, issuer_pubkey, signature`. The cron path stores `grant_snapshot` at registration and `validate_fire` re-checks signature + tool-in-scope + expiry at fire time (`agent-cron/src/scheduler.rs:159`). Render drift as a diff: snapshot column vs live column, any *widening* highlighted in danger.

**Design notes.** This is paper-zone (editable). Every default that is intentionally strict gets a one-line "why this is locked" so it reads as protection, not friction.

---

### 6.6 Health Strip — *L3, always-ambient Doctor + tripwires*

**Purpose.** A thin persistent strip, never more than a glance — the compliance heartbeat made ambient. Green when all checks pass, amber on `Warn`, red on `Blocker`.

**Renders (Doctor — the 11 real checks, `doctor/checks.rs`).** Click to expand the full **signed** report (`SignedDoctorReport { toml_body, attestation }`, `doctor/report.rs:85` — TOML body + ed25519 over the literal bytes). Each check is a row with `Severity` (`Pass|Warn|Blocker`):

1. `audit-chain-integrity` — `verify_integrity()` over the chain.
2. `audit-file-permissions` — Unix mode `0o600`.
3. `approval-queue-depth` — depth vs threshold (default 100).
4. `pending-break-glass` — Surfaced=Blocker, Unaffirmed=Warn.
5. `runtime-presence` — tokio runtime.
6. `capsule-manifest-reverify` — re-load `.cps`, re-check signature.
7. `wasm-linker-recheck` — `(manifest, WIT)` capability cross-check; mismatch=Blocker.
8. `policy-bundle-hash` — file SHA-256 vs pinned; drift=Blocker.
9. `tla-spec-ci-status` — TLA+ CI status; failing=Blocker, stale>48h=Warn.
10. `retention-age` — audit mtime vs max age (default 90d).
11. `anchor-reconciliation` — `is_anchored(record_hash)` for expected roots; missing=Blocker.

**Renders (Tripwires — the 9 real FedRAMP tripwires, `agent-cron/src/tripwires/jobs.rs`).** Nine small lamps. A fired lamp animates and **links to the on-chain firing record** (a `fire_tripwire` tx in the TripwireRegistry contract; firing_id derived from tripwire_id + scope + block). Each lamp: id, one-line condition, severity (Medium/High/Critical), state (`JobOutcome::{NoBreach, Fired(tx), Failed}`):

| Id | Fires on | Severity |
|---|---|---|
| `TRIP-AU-001` | RocksDB hot tier > 80% | Medium |
| `TRIP-AU-002` | Audit shipper backlog > 1000 | High |
| `TRIP-SC-001` | TLS cert expiry < 30 days | Medium |
| `TRIP-AC-001` | Account elevated > 4h continuous | High |
| `TRIP-CM-001` | Multi-sig method called by non-multi-sig acct | High |
| `TRIP-IA-001` | `requestElevation` without `auth_mode` | Medium |
| `TRIP-AC-002` | Status change → no auto-revoke within 5 blocks | **Critical** |
| `TRIP-AU-003` | IPFS pin failure rate > 1%/hour | **Critical** |
| `TRIP-SI-001` | Release manifest hash mismatch on deploy | **Critical** |

**Design notes.** Lives in the chrome at all times. Calm green by default; escalates only when it must. The expanded report straddles both zones: the *checks* are config-adjacent (paper), but the *firing records* are sealed-ledger (evergreen) and link on-chain.

---

### 6.7 Audit Scrubber — *L1/L3, replay — BUILD THIS FIRST*

**Purpose.** Open any past run as a **read-only composition** and scrub it. Because the chain is hash-linked and CBOR-canonical, each `AuditRecord` is a frame and the timeline reconstructs exactly. For an assessor, this is the single most valuable screen — a provable, frame-accurate replay of what the agent did and who approved it.

**Renders.** A timeline of `AuditRecord` frames (`audit/record.rs:93`): `sequence, timestamp, previous_hash, event_type (one of 19), payload, actor (DID), signatures: Vec<RoleSignature>, chain_anchor: Option<AnchorRef>`. Each `RoleSignature` row: `signer` (DID), `role`, `signed_at`, `surface` (`SigningSurfaceTag`). Anchored frames show an ⛓ "anchored on-chain" mark with the `AnchorStrategy`.

**The integrity proof, visualized.** Run `verify_integrity()` (`audit/chain.rs:126`) on load. A valid chain shows an unbroken green spine. **Tampering shows as a broken link** at the exact `sequence` where `previous_hash` mismatches — render the broken segment in danger with the error string ("previous_hash break at sequence N: chain tampered"). This is the runtime's verification surfaced visually; it is the assessor demo.

**Interactions.** Drag playhead backward/forward → step through frames (deterministic; this is *replay of recorded records*, not re-execution). Filter by `event_type`, `actor`, `role`. Export = an `AuditExport` event itself.

**States.** Verified (green spine) · Tampered (broken link, danger) · Partially anchored (some frames ⛓, some pending reconciliation — ties to doctor check #11). Loading = spine skeleton.

**Design notes.** This is the heart of the **sealed-ledger zone** — evergreen, heavier, "permanent." It must feel different from the editable canvas above it. Read-only is a feature: nothing here is draggable-to-mutate; you scrub, you don't edit.

---

### 6.8 Swarm View — *L1, multi-agent*

**Purpose.** When more than one agent runs, zoom out to a swarm board — each agent is a composition; dependencies drawn between them. Engage the swarm without overwhelm: **the swarm is just compositions one level up, with the same descent rule.** Zoom into any agent → back to a single Composition Canvas.

**Renders.** Each agent node = a Composition (a run + its chain). Edges = dependencies (e.g., an SOP step whose `condition` references another's output, `sop.rs:31`). Nested `SOPDefinition`s render as collapsible sub-compositions.

**Interactions.** Zoom into a node → Composition Canvas (6.1). Zoom out → board. The board is read-mostly; mutation happens inside a composition, never by rewiring the board arbitrarily (that would reintroduce node-graph sprawl — avoid it).

**Design notes.** Keep edges sparse and meaningful (dependency, not data-flow spaghetti). This is the *ceiling*; build it last (Gate 4).

---

### 6.9 Break-Glass Console — *L3, the emergency path*

**Purpose.** A deliberately stark, hard-to-reach surface for the SecurityOfficer break-glass flow. This is the one screen where the calm aesthetic gives way to gravity — it should feel heavy.

**Renders (the real state machine, `hitl/break_glass.rs`).**
- **Phase** (`BreakGlassPhase`): `Pending → Invoked → {Affirmed | Unaffirmed} → Surfaced`. Render as a weighted progress spine.
- **72-hour affirmation countdown** (`AFFIRMATION_WINDOW = 72h`, `break_glass.rs:31`) — a live timer from `invoked_at`. On expiry, phase flips to `Unaffirmed` (`tick`), then `Surfaced` onto a doctor report (`surface`).
- **Affirmation quorum progress:** `{Reviewer, ComplianceOfficer}` (`break_glass.rs:44`) — two slots. SO who invoked **cannot self-affirm** (enforced). Show who has affirmed and via which `SigningSurfaceTag`.
- **ITAR hard-block:** if the manifest touches ITAR (`manifest_touches_itar`, `break_glass.rs:339`), the break-glass path renders **greyed out and locked** — the runtime returns `BreakGlassError::ItarBlocked`. Make the lock and its reason unmistakable.

**States.** No invocation (the console is dormant/sealed) · Invoked (countdown running, all 5 roles notified) · Affirmed (resolved within window) · Unaffirmed (window blown — heavy danger) · Surfaced (sticky; appears on doctor, won't transition further).

**Design notes.** Hard to reach by design (behind a confirm + role check). Evergreen-to-danger palette. No playful motion.

---

### 6.10 Code Drawer — *L4, the floor*

**Purpose.** The deepest layer — the literal code. Closes the loop on "closer to the code at each layer."

**Renders.**
- The literal **`manifest.toml`** (round-trips to `Manifest`).
- **`capsule.wit`** (the interface).
- **Decoded calldata** for a chain write: selector + arguments, **byte-addressed** (the `chain_calls` allow-list entry resolves to a contract; the encoder shows the exact bytes that would hit chain `40204`).
- **Read-only WASM disassembly** of `capsule.wasm`.
- The `.cps` archive contents (`archive.rs`): `capsule.wit, capsule.wasm, procedure.md, manifest.toml, gherkin/*.feature, specs/*.tla, SIGNATURES/{publisher.sig, reviewer.sig}` — with which are required vs optional, and which are required for `tier-high+`.

**The live revalidation loop (the marquee L4 feature).** Editing the manifest here **re-runs the WIT/manifest cross-check live** (the same logic as doctor check #7 `wasm-linker-recheck` / `verify_capability_against_wit`): a capability declared in the manifest that no longer matches the WIT **lights red before the user can load it**. Likewise, a placeholder/blank `content_hash` shows the "unverified — pack & sign" state from §6.2. The drawer is where the manifest writes itself when you toggle capability glyphs in the Palette, and where it's proven consistent.

**Design notes.** Full-floor modal (`pop`). Mono everywhere. Byte offsets and selectors are the content, not decoration. This is read-first; the editable path (manifest edit → live revalidate) is the build target.

---

## 7. App shell, navigation, and the descent gesture

- **One app shell** (reuse the `AppShell` pattern from the design system): evergreen header chrome (sealed-zone identity), paper body (editable zone), persistent **Health Strip** (6.6) pinned in the chrome, and a collapsible right rail for the **Approval Dock** (6.4).
- **Descent is a consistent gesture.** Click *into a layer* → L2 Inspector lifts in. Click *into a property* → deepens toward L3/L4. A persistent breadcrumb shows current depth (L0…L4) so the user always knows how close to the bytes they are — and can ascend in one click.
- **Two workspaces** (Adobe-style), tuned by analytics over time: **Beginner** (L0/L1 only, twirls collapsed) and **Advanced** (twirls remembered, L3/L4 one gesture away). Default new users to Beginner.
- **The two zones are spatially fixed:** composition/config above (paper), sealed ledger below (evergreen). The Audit Scrubber and firing records always render in the lower/evergreen zone regardless of how you got there.

---

## 8. State & data contracts the UI subscribes to

The front end is a viewport; here is the state it reads and the intents it may submit. (Names are the frozen `lib.rs` surface plus the per-module primitives cited above.)

**Reads (subscriptions / queries):**
- Pending approvals: `ApprovalQueue::signatures_on(call_id) -> Vec<Signature>`, `payload_for(call_id) -> Option<Vec<u8>>`, `PendingView`.
- Run/composition: the ordered call sequence + live step pointer (Composition Canvas); per-capsule `ToolResult`.
- Capsule facts: `Manifest` (all 8 sections) per capsule; verified/unverified flag from `capsule_body_verified`.
- Audit: iterate `AuditChain` records; `verify_integrity() -> Result<u64, _>`.
- Doctor: `SignedDoctorReport` (11 checks + `Severity`).
- Tripwires: per-tripwire `JobOutcome` + on-chain firing tx refs.
- Break-glass: `BreakGlassPhase` + `invoked_at` + affirmation set per action.
- Grants: `CapabilityGrant` snapshot vs live (drift).

**Intents (submissions — the core decides):**
- `add_signature(call_id, AttestedSignature)` — attest to exact `payload` bytes (hash-pinned).
- `submit_for_action(call, payload, quorum, proposer)` — enqueue a gated action (the canvas does this when the playhead hits a keyframe).
- reject / set-oversight-dial (per-capsule mode mask) / set session auto-approve (TTL-bounded, Low/Medium only).
- break-glass `invoke(action, sig, manifest)` (SO), `affirm(action, sig)` (Reviewer|CO).
- roster `authorize(pubkey, role)`.
- capsule add-to-run / reorder / dry-run-toggle / solo.

**The boundary restated:** none of these intents *grant* anything. The UI submits; `ApprovalQueue`, `Quorum`, `SignerRoster`, `CapsuleDispatch`, and the break-glass machine return the outcome. Design every confirm flow as "submit & await core verdict," never "do it."

---

## 9. Cross-cutting visual systems (define once, reuse)

1. **Risk-tier system** — the color + keyframe-shape + approval-presentation triple (§3.1, §6.4). The most repeated signal in the product.
2. **Role/quorum grammar** — five role glyphs (Operator, Reviewer, ComplianceOfficer, SecurityOfficer, Auditor); signature states ✓ (signed) / ⧗ (awaited) / ⊘ (conflicted-disabled). Auditor glyph always non-approving. CO/SO shown as mutually exclusive when both are candidate signers.
3. **Data-class taxonomy** — seven chips; ITAR heaviest (it gates break-glass); FERPA has three sub-variants.
4. **Capability glyph row** — seven `CapabilityToken` glyphs; the same row appears on palette tiles, canvas clips, and the Inspector capabilities twirl.
5. **Signing-surface badges** — five tags; small provenance marks on every signature row.
6. **Anchoring marks** — ⛓ for on-chain-anchored frames + the strategy (PerCapsule/PerApproval/NightlyMerkle).
7. **The two-zone treatment** — paper (editable) vs evergreen (sealed). The single most important macro-decision in the layout.

---

## 10. Build sequence — four gates, smallest provable slice first

1. **Gate 1 — Read-only Composition Canvas + Audit Scrubber (6.1 read + 6.7).** Render a past run as a layer stack, scrub it, verify the chain (`verify_integrity`). No execution, no new trust surface. **This is the assessor demo and it rides entirely on shipped code — ship it first.** Screens: Audit Scrubber (verified + tampered states), read-only canvas, the two-zone shell, Health Strip (static).
2. **Gate 2 — Inspector + Capsule Palette, read (6.3 + 6.2).** Click into layers; browse the signed library; see capabilities, data classes, provenance; show the locked/unverified tiles. Still no execution. Proves progressive disclosure end-to-end.
3. **Gate 3 — Approval Dock + live execution (6.4 + 6.1 live).** Wire the canvas to live `CapsuleDispatch`; surface the four risk-tier presentations; light the playhead; render the hash-pin and live `signatures_on`. First writeable slice; rides existing gates. *(Note: full L0 chat depends on the agent-loop/model-resolver stubs landing — Gate 3 can run capsule/SOP runs without free-form chat.)*
4. **Gate 4 — Trust Panel + Code Drawer + Swarm View + Break-Glass (6.5 + 6.10 + 6.8 + 6.9).** The expert floor and the multi-agent ceiling, plus the emergency path. Last, because everything beneath must be solid.

Build Gate 1 against the **native shell** (where the audit files and `0o600` perms live), then port components to the **web shell** — shared `cdylib` core makes that a re-skin, not a rebuild. Web shell routes High/Critical signatures to attested hardware (`SigningSurfaceTag::LocalWeb` for the surface, hardware for the key).

---

## 11. Implementation reality (so you don't mock vapor)

| Subsystem | Status | Implication for design |
|---|---|---|
| HITL: `ApprovalQueue`, `Quorum`, `SignerRoster`, SoD, hash-pin | ✅ built | Design the dock against real states; everything in §6.4 is backed. |
| Break-glass machine (phases, 72h, affirmation, ITAR block) | ✅ built | §6.9 is fully real. |
| Capsule: `Manifest`, verify/integrity gate, `.cps` archive, linker, tiers | ✅ built | §6.2/6.3/6.10 inspector + palette are real; unverified-dimming is a real gate. |
| Audit chain: records, `verify_integrity`, CBOR, FS + chain sinks | ✅ built | §6.7 scrubber is real — *make this the demo*. |
| Doctor (11 checks, signed report) + Tripwires (9) | ✅ built | §6.6 Health Strip is real, names exact. |
| Cron scheduler + SOP engine + grant snapshot (AGT-14) | ✅ built | §6.5 grant-drift + §6.8 SOP nesting are real. |
| Tool inventory (11 chain + 6 code) | ✅ built | §6.2 palette content is real, risk levels exact. |
| **Agent loop** (`agent/mod.rs`) | ⚠️ **stub** (~9 lines, CIT-AGENT-3) | **L0 chat / streaming output cards: design the surface, treat live data as forthcoming.** Don't block Gates 1–2 on it. |
| **Model resolver** (`model/mod.rs`) | ⚠️ **stub** (~10 lines, CIT-AGENT-3) | Model-picker UI is speculative until this lands (planned: Ollama:11434, llama.cpp:8080/8000, embedded Gemma GGUF, SHA-256 model verification). |

---

## 12. Open questions for the user / designer (decide before Gate 3)

1. **Native shell choice:** continue the existing Slint path (`citrate-gui-native`'s `citrate_gui_native`, which already consumes `agent-core` and `ApprovalQueue`) or wrap the same `cdylib` core in a Tauri shell? Slint is the shipped path and already drives approvals; Tauri buys web-component reuse. (Affects how 1:1 the native and web component sets can be.)
2. **L0 timing:** ship L0 chat as a designed-but-stubbed shell in Gate 1 (so the depth model reads top-to-bottom from day one), or hide L0 until the agent loop lands? Recommendation: show a calm L0 placeholder that descends into the real L1, so the metaphor is whole.
3. **Reorder semantics:** confirm drag-to-reorder edits a *run plan* (intent) rather than the capsule set on disk (see §5 note) — this changes the canvas's data model.
4. **Theme default:** dark-first (evergreen, matches explorer) or paper-first (matches the "government-document calm" of the buyer webapp)? The two-zone rule works in both; pick the default.

---

## 13. One-line summary

A creative tool that hides the (stubbed) transformer at the top and reveals the (real) calldata at the bottom — where every layer down trades one abstraction for one truth, where the front end only ever submits intents and the Rust core makes every decision, and where the compliance the runtime already enforces — the hash-pinned approvals, the quorum lattice, the 11 doctor checks, the 9 tripwires, the frame-accurate audit replay — becomes the most beautiful thing on screen instead of the most buried.

---

### Appendix A — Primitive → file index (for the designer's engineering counterpart)

```
Manifest / RiskTier / Role / DataClass / NetworkPolicy / SigningTier
                                    agent/core/src/capsule/manifest.rs
CapabilityToken (linker glyphs)     agent/core/src/capsule/linker.rs:36
Capsule verify / integrity gate     agent/core/src/capsule/dispatch.rs:88-115, 230-253
.cps archive format                 agent/core/src/capsule/archive.rs
Signing tiers / signature verify    agent/core/src/capsule/tiers.rs
Quorum / for_tier / satisfied_by    agent/core/src/hitl/quorum.rs
Roles / SoD (is_conflict, can_approve) agent/core/src/hitl/roles.rs
SignerRoster / signer_id            agent/core/src/hitl/signing.rs
ApprovalQueue / hash-pin / signatures_on / payload_for
                                    agent/core/src/hitl/mod.rs
Break-glass machine / 72h / ITAR    agent/core/src/hitl/break_glass.rs
SigningSurfaceTag / AuditRecord / EventType (19) / RoleSignature
                                    agent/core/src/audit/record.rs
verify_integrity                    agent/core/src/audit/chain.rs:126
Audit sinks (FS 0o600 / chain anchor strategies)
                                    agent/core/src/audit/sink/{fs,chain}.rs
Anchor client (chain_id 40204)      agent/core/src/chain/anchor.rs
Doctor (11 checks) / Severity / SignedDoctorReport
                                    agent/core/src/doctor/{checks,report}.rs
Tripwires (9) / JobOutcome          agent-cron/src/tripwires/{jobs,evaluator}.rs
SOPDefinition / SOPTrigger / SOPStep agent-cron/src/sop.rs
CronJob / grant_snapshot / validate_fire (AGT-14)
                                    agent-cron/src/scheduler.rs
CapabilityGrant / PolicyProfile     agent-legacy/src/canonical.rs
ApprovalFlow / auto-approve TTL 3600s agent-legacy/src/approval.rs
Budget / EmergencyStop / delegation agent-legacy/src/{budget,estop,delegation}.rs
CodeAgentBridge (mandatory approval) agent-code/src/bridge.rs
Chain tools (11) / Code tools (6)   agent-chain/src/tools/, agent-code/src/tools/
Frozen public API surface           agent/core/src/lib.rs
Core is rlib + cdylib               agent/core/Cargo.toml
```

### Appendix B — Design-system source files

```
Tokens (light/dark, motion, shadow) citrate-explorer/src/scan/scan.css
Tailwind ramp tokens                citrate-buyer-webapp/tailwind.config.ts
Voice / nomenclature / principles   citrate-explorer/DESIGN_BRIEF.md
                                    citrate-buyer-webapp/DESIGN_AGENT_BRIEF.md
Brand marks (#8ecc09 green, #ffbd10 yellow) branding/
Component foundation                shadcn/ui on Tailwind (extend, don't fork)
```
