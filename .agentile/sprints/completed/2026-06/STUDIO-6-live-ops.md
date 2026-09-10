---
created: 2026-06-04T03:00:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: archived
sprint: STUDIO-6
---

# STUDIO-6 — Live operation

**Goal.** Move the remaining `core-live` execution surfaces from modeled to real:
the async `ApprovalQueue` consuming STUDIO-4's real signatures, real chain reads +
anchoring against `rpc.citrate.ai` (chain 40204), and a real signed `DoctorReport`.

**Why now.** STUDIO-4 produces real ed25519 attestations; STUDIO-3 deferred the
execution surfaces that *consume* them. This sprint closes that loop end-to-end.

**The exact bridge (verified).** `studio::signing` and `agent-core::hitl::signing` use
the identical scheme: `SigningKey::from_bytes(seed).verifying_key()` and
`signer_id = SHA-256(pubkey)`. So a studio `EnrolledSigner.secret` →
`Ed25519FileSurface::from_seed(secret, role)` produces an `AttestedSignature` the real
`verify_attestation` + `StaticSignerRoster` + `ApprovalQueue::add_signature` accept. No
adaptation — the keys line up by construction.

**Scope (achievable real, this sprint).**
1. **Async `ApprovalQueue`** (`core-live`): `core_bridge::hitl::LiveQueue` wraps a tokio
   runtime + `ApprovalQueue` + a `StaticSignerRoster` built from the enrolled roster.
   A High gate is submitted; studio's enrolled keys sign; the real
   verify_attestation / RM-G.1 roster auth / SoD / `Quorum::satisfied_by` decide it.
   End-to-end proving test.
2. **Chain reads** against `rpc.citrate.ai` (40204): a `chain` module doing real
   JSON-RPC (block height / anchor existence) — surfaced honestly (live vs offline).
3. **`DoctorReport`** (`core-live`): run the real doctor against a constructed context;
   surface real `Severity` results.

4. **`CapsuleDispatch`** (`core-live`): REAL — the runtime ships 10 prebuilt WASM
   capsules in `citrate-agent-runtime/capsules/` (`hello`, `echo-chain`, …). Point
   `CapsuleDispatch::load_from_dir` at that dir and `call_raw` the `hello` capsule's
   `greet` export through wasmtime — real execution, real `ToolResult`.

**Honest boundaries (stated, not faked).**
- **Chain *writes* / anchoring** need a funded signer + gas; reads are unauthenticated
  and land this sprint, writes are gated on a key/funding seam.
- The L0 first-prompt agent loop is CIT-AGENT-3 (upstream, not landed) — local fallback
  stays, clearly labelled.

**Plan.**

| # | Step | Status |
|---|---|---|
| 1 | `core_bridge::approvals::LiveQueue` + end-to-end proving test | done |
| 2 | Wire the dock gate through `LiveQueue` under `core-live` | → next (proven, not yet the live UI path) |
| 3 | `chain` module: real reads vs `rpc.citrate.ai` (40204) | done |
| 4 | Real `DoctorReport` (core-live) + `report_rows` for the Health Report | done |
| 5 | `CapsuleDispatch` — REAL: load_from_dir + call_raw the `hello` capsule | done |
| 6 | Tests + screenshots; default + core-live green | done |

**Daily updates.**

- 2026-06-04 — kickoff. Mapped the agent-core APIs; building the `ApprovalQueue` bridge
  first (the centerpiece STUDIO-4 was built toward).
- 2026-06-04 — **all four live surfaces real.** (1) `LiveQueue` wraps the async
  `ApprovalQueue` behind a sync surface; studio's enrolled keys → `Ed25519FileSurface`
  → real attestations → real verify + RM-G.1 roster auth + SoD + quorum. End-to-end test
  green. (3) `chain` reads `rpc.citrate.ai` for real — `eth_chainId` = 0x9d0c (40204),
  live block in Settings → Policy. (5) `CapsuleDispatch::load_from_dir` over the repo's
  10 prebuilt capsules; `call_raw` the `hello` capsule → real `"Hello, Aleia"` via
  wasmtime. (4) `core_bridge::doctor` runs the real checks against a real temp audit
  chain — integrity "verified 3 records", permissions "owner-only 0600". 37 core-live /
  34 default tests; both builds zero-warning (studio code).

**Decisions made.**

- **`LiveQueue` = a sync wrapper over the async queue** (tokio runtime holds the parked
  `submit_for_action`; `add_signature` is sync; approval detected by entry removal). The
  poll-until-registered loop is the one inelegant seam (upstream "register then await"
  would remove it).
- **The shipped capsules are unverified** (placeholder content-hashes; CIT-AGENT-3e), so
  dispatch sets `CITRATE_ALLOW_UNVERIFIED_CAPSULES` — a loudly-logged dev opt-in, never a
  silent bypass.
- **Chain reads now; writes deferred** behind `anchor_available()` (a funded
  `CITRATE_ANCHOR_KEY` seam) — reads are unauthenticated, writes need gas.
- **Live-dock rewiring to `LiveQueue` is the next step** — the queue is proven + available;
  the running UI still decides via STUDIO-3's policy seam.

**Exit criteria.**

- [x] A real ed25519 signature from an enrolled key satisfies a real High gate through
      the real async `ApprovalQueue`, end to end (`high_gate_satisfied_by_real_enrolled_signatures`).
- [x] Real chain reads hit `rpc.citrate.ai` (40204); offline surfaced ("unreachable").
- [x] A real `DoctorReport` (core-live) with real `Severity` against a real audit chain.
- [x] A real `hello` capsule dispatches through wasmtime → `"Hello, Aleia"`.
- [x] Honest seam for chain writes (`anchor_available()` / `CITRATE_ANCHOR_KEY`), documented.
- [x] Default build green (34 tests, 0 warnings); core-live green (37 tests).

**Close note.**

STUDIO-6 makes the execution real. Under `core-live`, all four live surfaces are the
runtime's own: a studio enrolled key's signature flows through the real async
`ApprovalQueue` (verify + RM-G.1 roster auth + SoD + quorum) to satisfy a real High gate;
a real `hello` capsule executes through wasmtime (`"Hello, Aleia"`); chain reads hit the
live `rpc.citrate.ai` (chain 40204, advancing block); and a real `DoctorReport` runs the
runtime's checks against a real audit chain. The STUDIO-4 bet paid out exactly — because
both sides compute `signer_id = SHA-256(pubkey)`, the queue integration needed no adapter.
The honest edges, named and seamed: the live dock still decides via STUDIO-3's policy seam
(rewiring it to `LiveQueue` is the next step — the queue is proven + available); chain
*writes*/anchoring need a funded signer (gated); signed-capsule dispatch waits on the
upstream packer (CIT-AGENT-3e), so today's real execution rides a loudly-logged dev
opt-in; the L0 agent loop is CIT-AGENT-3. Retro + journal + essay alongside.
