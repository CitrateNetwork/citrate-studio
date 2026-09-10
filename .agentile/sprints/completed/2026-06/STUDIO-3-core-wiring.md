---
created: 2026-06-04T22:40:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: archived
sprint: STUDIO-3
---

# STUDIO-3 — Wire the real `citrate-agent-core`

**Goal.** Replace the in-process `RunState` model with the real
`citrate-agent-core`, subsystem by subsystem, with no `.slint` change — proving the
STUDIO-1 trust boundary makes this a swap, not a rewrite.

**Why now.** Auth (STUDIO-2) gives us *who*; the real core gives us *what actually
happens*. The single most important property to demonstrate first is that the policy
the UI renders (quorum, separation-of-duties) comes from the **core**, not from
`main.rs`.

**Approach — incremental, behind a feature gate.** `citrate-agent-core` pulls
`wasmtime 45` and an SSH-sourced `citrate-wallet-core` (rev cached locally). To keep
the default build fast and green while the integration matures, the dep is **optional,
behind a `core-live` feature**. Each subsystem migrates behind that flag; when all are
live, `core-live` becomes default and the modeled core is deleted.

Verified pre-conditions (2026-06-04): the real policy APIs are public —
`hitl::{Quorum, is_conflict, can_approve}`, `capsule::manifest::{Role, RiskTier,
DataClass}`, `audit::AuditChain` (`verify_integrity`), `doctor::{DoctorReport,
Severity}` — and the pinned `citrate-wallet-core` rev is in the local cargo git cache,
so the dep resolves offline despite the missing `github-citrate-chain` SSH alias.

**Scope (this sprint — the start).**
- Add `citrate-agent-core` as an optional path dep + the `core-live` feature.
- `core_bridge.rs` (gated): map studio's string roles/tiers → the core's `Role`/
  `RiskTier`, and expose the real `is_conflict` / `can_approve` / `Quorum::for_tier`.
- A `policy` seam: default = the current faithful hand-rolled SoD/quorum; `core-live` =
  the real core. `approval_roster()` computes through `policy::*`.
- Default build green (zero warnings); `core-live` build validated to compile.

**Scope (later steps — same sprint or STUDIO-3b).**
- Audit: render a real `AuditChain`; `verify_integrity()` drives the scrubber verdict.
- Approvals: route through the real async `ApprovalQueue` (`submit_for_action`,
  `add_signature`, `signatures_on`, `payload_for`) with real `AttestedSignature`.
- Doctor: the real signed `DoctorReport` + `Severity` into the Health Strip.
- Dispatch: the run plan → real `CapsuleDispatch::call_raw` over installed `.cps`.
- Recorder: anchor via `RecorderClient` on chain 40204.

**Out of scope.** The agent loop / model resolver (runtime's CIT-AGENT-3); packaging.

**Repos affected.** `citrate-studio` (+ a read-only path dep on
`citrate-agent-runtime/agent/core`).

**Plan.**

| # | Step | Status |
|---|---|---|
| 1 | Optional `core-live` dep + feature | done |
| 2 | `core_bridge` — role/tier mapping + real SoD/quorum | done |
| 3 | `policy` seam; route `approval_roster` through it | done |
| 4 | Default build green; validate `core-live` compiles + runs | done |
| 5 | Audit chain `verify_integrity` wired | done |
| 6 | Real quorum **decision** (`Quorum::satisfied_by`) | done |
| 7 | Stateful async `ApprovalQueue` | → STUDIO-6 (live execution) |
| 8 | `DoctorReport` (real context), `CapsuleDispatch`, `RecorderClient` | → STUDIO-6 (live execution) |

**Scope refinement (decided during execution).** STUDIO-3's true, coherent scope is
*"the real core owns every **pure policy + verification** decision the UI renders"* —
done: SoD (`is_conflict`), the quorum tier shape + count (`Quorum::for_tier`), the
quorum **decision** (`Quorum::satisfied_by`), and audit integrity
(`AuditChain::verify_integrity`). These are decidable in-memory and are now real under
`core-live`. The remaining surfaces are **execution / live-deployment inspection**, not
pure policy, and require a real runtime environment that the studio demo does not have:
- the **stateful async `ApprovalQueue`** (real ed25519 attestations over real payloads,
  the async resolver/timeout) — exercised when studio submits to a *live* core;
- **`DoctorReport`** with a real `DoctorContext` (audit files, capsule dir, policy
  bundle, TLA CI, chain anchors) — inspects a live deployment;
- **`CapsuleDispatch`** over real signed `.cps` (wasmtime instantiation) and
  **`RecorderClient`** anchoring on chain 40204 — execute against live capsules/chain.

These move to **STUDIO-6 (live operation)** + **STUDIO-5 (from-scratch setup)**, where the
environment exists. This is a refinement, not a cut: the gate's thesis — *the front end
never decides; the real core does* — is fully proven for the policy surfaces, which is
what STUDIO-3 set out to show.

**Daily updates.**

- 2026-06-04 — kickoff. Confirmed agent-core's policy surface is public and the
  wallet-core rev is cached (build feasible offline). Registered the `citrate-studio`
  OIDC client (STUDIO-2 dependency). Added the `core-live` feature + `core_bridge` +
  `policy` seam; routed the approval-roster SoD/quorum through it.
- 2026-06-04 — **test backfill** (method state-check). The closed STUDIO-1 shipped
  with no automated tests (its retro flagged the debt); only auth had coverage.
  Extracted the playback step into a testable `RunState::tick` and added a `tests`
  module: the run state machine (gate pause + side-effects, completion + tripwire),
  `quorum_met`, `approval_roster` SoD, the `policy` seam, and a data-catalog sanity
  check. 12 tests green (6 auth + 6 new). The `policy::*` tests are the parity guard
  between the default rules and the real core — both feature builds must pass them
  identically. Repays STUDIO-1's debt without editing the closed sprint, and covers
  STUDIO-3's new policy logic. Going forward, tests land within each sprint.
- 2026-06-04 — **validated.** `cargo build --features core-live` compiles and links
  against the real `citrate-agent-core` v0.4.0 (wasmtime 45 + cranelift + the cached
  SSH wallet-core; 3m55s, studio code warning-free). The real-core binary runs: the
  gate-state snapshot renders the quorum card with "NofM · 2 of 3" sourced from the
  real `Quorum::for_tier(High, …)` and SoD from `hitl::is_conflict` — pixel-identical
  to the modeled build. The swap is proven, not speculative. Default build stays green
  (zero warnings) for fast iteration. Steps 5–8 (audit/approval-queue/doctor/dispatch)
  are next.

- 2026-06-04 — **step 5: real `AuditChain::verify_integrity` in the scrubber.** Added an
  `audit_verify` seam (default modeled; `core-live` = `core_bridge::audit`). The bridge
  builds a real `AuditChain` from the scrubber frames over an in-memory `AuditSink` (the
  minted Genesis + 11 appended events), and the "Simulate tamper" affordance corrupts a
  record's payload so the hash chain *genuinely* breaks — verdict + break sequence come
  straight from `verify_integrity`. The scrubber now renders `scrubber-verdict` /
  `scrubber-ok` / `scrubber-break-seq` instead of a hardcoded string. Under `core-live`
  the verdict is the runtime's real error verbatim ("audit: previous_hash break at
  sequence 6: chain tampered"); the modeled default matches sans the `audit:` prefix —
  the break-seq parse handles both. New parity test `audit_verify_clean_and_tampered`
  passes under **both** builds (13 tests). Default build zero warnings; studio code clean
  under `core-live` (the 3 warnings are agent-core's own `insecure-dev-hitl` cfg).

**Decisions made.**

- Feature-gate the heavy dep (`core-live`, off by default) and migrate subsystems
  behind it, rather than a big-bang swap — keeps the app shippable throughout.

- 2026-06-04 — **step 6: real quorum decision.** Extended the policy seam with
  `quorum_satisfied(tier, required, signed)` and routed `quorum_met` through it. Under
  `core-live` it is the real `Quorum::for_tier(tier, required).satisfied_by(&signed)` —
  the authoritative decision: NofM needs n hits from the required set, Critical needs the
  fixed {SO, CO, Reviewer} multiset, and non-approving roles (Auditor) never count. New
  parity test passes under **both** builds (**14 tests**); default zero warnings. With
  this, every *pure policy + verification* decision the UI renders — SoD, quorum shape,
  quorum decision, audit integrity — is the real core's under `core-live`.

**Exit criteria.**

- [x] `core-live` feature + optional dep declared; default build green, zero warnings.
- [x] `policy` seam routes SoD + quorum **shape, count, and decision**; `core-live`
      delegates to the real core (`is_conflict`, `can_approve`, `Quorum::for_tier`,
      `Quorum::satisfied_by`).
- [x] Audit integrity is the real `AuditChain::verify_integrity` under `core-live`.
- [x] `cargo build --features core-live` compiles + runs; studio code warning-free.
- [x] Parity tests guard default ↔ core-live agreement (14 tests, both builds).
- [x] Execution surfaces (async queue, env-Doctor, dispatch, recorder) re-scoped to
      STUDIO-6 with rationale.

**Close note.**

STUDIO-3 wired the real `citrate-agent-core` for every **pure policy + verification**
surface the UI renders — separation-of-duties, the quorum tier shape, the quorum count,
the quorum **decision**, and audit-chain integrity — all behind a `core-live` feature so
the default build stayed fast and shippable the whole way. Each is guarded by a parity
test that passes under *both* the modeled default and the real core, and validated
end-to-end (the scrubber shows the runtime's genuine `verify_integrity` error string; the
quorum card's "2 of 3" comes from the real `Quorum::for_tier`). The trust-boundary thesis
— *the front end never decides; the real core does* — is proven, with zero `.slint`
change, exactly as STUDIO-1's design predicted. The honest boundary discovered during
execution: the remaining surfaces (stateful async `ApprovalQueue`, `DoctorReport` with a
real context, `CapsuleDispatch` over real `.cps`, `RecorderClient` chain anchoring) are
**execution / live-deployment inspection**, not pure policy, and need a runtime
environment the studio demo lacks — they move to STUDIO-6 (live operation) + STUDIO-5
(from-scratch setup). Retro: `docs/retrospectives/2026-06-04_STUDIO-3.md`.
