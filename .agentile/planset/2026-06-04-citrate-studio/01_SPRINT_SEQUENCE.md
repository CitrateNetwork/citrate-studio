---
created: 2026-06-04T21:50:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
sprint: citrate-studio roadmap
status: draft
---

# 01 — Sprint Sequence

> From "faithful shell over a modeled core" (STUDIO-1, done) to "a new user sets up the
> agent and runs it from scratch" (`v1.0.0`). Estimates are engineering-days for a
> focused operator+agent pair; acceptance criteria are the gate to close each sprint.

## Phase A — Identity & the real core (→ v0.2.0)

### STUDIO-2 — Authentication (OIDC + SIWE)  · ~3–4 days · IN FLIGHT
**Goal.** An operator signs in with their Citrate identity; Studio holds a validated
session and surfaces "signed in as 0x…".
**Approach.** Loopback PKCE (RFC 8252) against `citrate-identity` (`auth.citrate.ai`),
SIWE (EIP-4361) wallet signature, RS256 ID-token validation against `/jwks`, tokens in
the OS keyring, refresh + logout + SSE logout-cascade. See
`ADR-2026-06-04-auth-oidc-siwe.md` and `STUDIO-2-auth.md`. The compiling scaffold
(`src/auth.rs`: PKCE, token model, `TokenStore`, `signer_id`) lands first; the network
client (loopback server + token exchange + JWKS) lands behind a small dep set
(`reqwest`/`tiny_http`/`jsonwebtoken`/`keyring`) gated for the auditor's review.
**Acceptance.**
- [ ] Sign-in opens the system browser, completes SIWE, returns to a local callback,
      exchanges the code, validates the ID token, and stores it in the keyring.
- [ ] A Sign-In Slint surface + "signed in as 0x…" in the chrome.
- [ ] Logout revokes server-side and clears local tokens.
- [ ] `cargo deny` green on the new deps; `audit.toml` updated with rationale.

### STUDIO-3 — Wire the real `citrate-agent-core` (`cdylib`)  · ~5–7 days
**Goal.** Replace the in-process `RunState` model with the real core. The UI does not
change — only the source of its state.
**Approach.** Depend on `citrate-agent-core` (the `rlib`/`cdylib` already built by the
runtime). Bind the frozen surface: `ApprovalQueue` (`signatures_on`, `payload_for`,
`add_signature`, `submit_for_action`), `CapsuleDispatch::call_raw`, `AuditChain` +
`verify_integrity`, `Doctor`, `RecorderClient`. Map core types → the Slint structs the
UI already consumes (`ClipData`, `ApprovalRow`, `FrameData`, `DoctorCheck`,
`Tripwire`). The SoD/quorum the UI shows now comes from `Quorum`/`roles` in the core,
not from `main.rs`.
**Acceptance.**
- [ ] Approvals route through the real `ApprovalQueue`; a signature is a real
      `AttestedSignature` over the real payload; the hash-pin voids on payload change.
- [ ] The Audit Scrubber renders real `AuditChain` records; `verify_integrity()` drives
      the verdict; tamper is detected, not simulated.
- [ ] The Health Strip shows the real signed `Doctor` report + the real tripwire state.
- [ ] No `.slint` file changed for policy; the boundary holds.

## Phase B — From-scratch setup (→ v0.3.0)

### STUDIO-4 — Signer roster + key enrollment  · ~4–5 days
**Goal.** Turn an authenticated identity into a role-bearing signer with real keys.
**Approach.** Settings → Signer Roster becomes real: enroll a wallet's ed25519 pubkey
per role into `StaticSignerRoster`; `signer_id = SHA-256(pubkey)`. Signing surfaces in
tiers: `FileBacked` (dev) → OS keyring → PIV/CAC + FIDO2 (attested hardware, the
`SigningSurfaceTag::Slint` native path the spec calls for). Enforce the fail-closed
truth (no roster ⇒ High/Critical cannot be approved) in the UI.
**Acceptance.**
- [ ] Enroll/disenroll a signer; persist the roster; SoD candidates reflect it.
- [ ] At least file + keyring surfaces working; PIV/FIDO2 stubbed with a clear seam.
- [ ] A real signature from an enrolled key satisfies a real High gate end-to-end.

### STUDIO-5 — New-user setup that persists  · ~4–5 days
**Goal.** Onboarding stops being theatre: each step configures something real and the
config survives a restart.
**Approach.** Wire the 6 onboarding steps to real actions + a persisted
`citrate-studio.toml` (+ keyring for secrets): workspace/tenant, model-runtime
discovery (Ollama :11434 / llama.cpp :8080 / embedded Gemma, with SHA-256 verify),
roster enrollment (STUDIO-4), capsule install (signed `.cps` from a source, fail-closed
on unverified), oversight default. First launch → onboarding; thereafter → Studio.
**Acceptance.**
- [ ] A clean machine → onboarding → a working, persisted configuration.
- [ ] Re-launch loads the saved config; no re-setup.
- [ ] Model discovery + SHA-256 verification real (or honestly "planned" if
      CIT-AGENT-3's resolver isn't landed — surfaced, not faked).

## Phase C — Live operation (→ v0.4.0)

### STUDIO-6 — Live data + real runs  · ~6–8 days
**Goal.** The Composition Canvas drives a *real* run; the playhead reflects real
dispatch, not a timer.
**Approach.** The run plan → real `CapsuleDispatch` over installed capsules; the
playhead/firing/done states derive from real step completion; output cards show real
`ToolResult`. Real chain reads (40204) for balances/contracts; anchoring via
`RecorderClient`. L0 chat wires to the real agent loop/model resolver *when CIT-AGENT-3
lands* (until then, the local fallback stays, clearly labelled).
**Absorbs from STUDIO-3 (scope refinement, 2026-06-04).** STUDIO-3 wired the real core's
*pure policy + verification* surfaces (SoD, quorum shape/count/decision, audit integrity).
The *execution* surfaces it surfaced — the **stateful async `ApprovalQueue`** (real
ed25519 attestations + the async resolver), the **`DoctorReport`** against a real
`DoctorContext`, **`CapsuleDispatch`** over real `.cps`, and **`RecorderClient`** anchoring
— need a live runtime environment and land here (roster/keys come from STUDIO-4, the
environment from STUDIO-5). See `sprints/completed/2026-06/STUDIO-3-core-wiring.md`.
**Acceptance.**
- [ ] A real capsule run advances the canvas and lands real records in the ledger.
- [ ] Chain reads/anchors hit 40204; the Anchor marks are real.
- [ ] A real ed25519 signature from an enrolled key satisfies a real High gate through
      the real async `ApprovalQueue`, end to end.
- [ ] The Health Strip shows a real signed `DoctorReport` against the live environment.

### STUDIO-7 — Hardening pass *(inserted; not in the original sequence)* · done
**Goal.** Button up every edge: remove all `#[allow(dead_code)]`, wire test-only bridges
into the UI, triage every `unwrap`, label/gate every seam. See
`sprints/completed/2026-06/STUDIO-7-hardening.md`. The original "interactivity + e2e" and
"packaging" sprints shift to STUDIO-8 and STUDIO-9 below.

### STUDIO-8 — Interactivity + e2e  · ~4–5 days
**Goal.** The deferred tactile features + a real test harness.
**Approach.** Drag-to-reorder the run plan (intent), eye/solo dry-runs wired to real
dispatch flags, Code Drawer live manifest edit → real `wasm-linker-recheck`, the Swarm
View (multi-agent), and a Slint-driven e2e harness (drive clicks, assert state) — the
gap the STUDIO-1 retro flagged.
**Acceptance.**
- [ ] Reorder/dry-run/solo affect real dispatch.
- [ ] e2e harness drives the gate→sign→resume→audit loop headlessly and asserts it.

## Phase D — Ship (→ v1.0.0)

### STUDIO-9 — Packaging, signing, release  · ~3–4 days
**Goal.** A signed, installable build; the Tier-1 audit gate.
**Approach.** Native installers (macOS `.dmg`/notarized, Windows MSI, Linux AppImage);
the `cdylib` shared core proven to back both this and the web re-skin; release CI (the
runtime's `release.yml` pattern). Extract the UI kit (`theme`/`typography`/`icons`/
`primitives` + the snapshot harness) into a shared crate for `gui-native`.
**Acceptance.**
- [ ] Signed installers on the 3 desktop targets.
- [ ] UI kit extracted + consumed by at least one other shell.
- [ ] External audit attestation against the release SHA (`AUDIT_TIER.md`).

## Total

| Phase | Sprints | Est. (eng-days) |
|---|---|---|
| A — identity & real core | STUDIO-2, 3 | ~8–11 |
| B — from-scratch setup | STUDIO-4, 5 | ~8–10 |
| C — live operation | STUDIO-6, 7 | ~9–11 |
| D — ship | STUDIO-8 | ~3–4 |
| **Total** | **7 sprints** | **~28–36 eng-days** |

## Risks + mitigations

- **CIT-AGENT-3 (agent loop / model resolver) not landed.** L0 chat + model discovery
  depend on it. *Mitigation:* keep the local fallback, label it "forthcoming," and
  gate STUDIO-6's chat wiring on the runtime sprint — don't fake it.
- **`citrate-identity` device-flow incomplete (S4).** *Mitigation:* use the
  production-ready loopback PKCE path (RFC 8252) that explorer/dashboard already use.
- **Auth deps expand the audit surface.** *Mitigation:* land the pure scaffold first;
  add `reqwest`/`keyring`/`jsonwebtoken` in one reviewable commit with `cargo deny` +
  `audit.toml` rationale.
- **`cdylib` FFI friction.** *Mitigation:* consume `citrate-agent-core` as an `rlib`
  Rust dep (same process), not over a C ABI — the UI is a Rust app, not a foreign host.
