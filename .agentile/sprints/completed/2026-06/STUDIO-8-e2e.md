---
created: 2026-06-04T06:00:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: archived
sprint: STUDIO-8
---

# STUDIO-8 — Interactivity + e2e harness

**Goal.** Close the test-quality debt the STUDIO-1 retro flagged: a headless harness that
drives the full operator loop — **play → gate pause → sign to quorum → resume → run to
done → audit verifies** — and asserts the state at every transition, through the *real*
intent code and the real `refresh → AppState` bridge. Plus wire the deferred dry-run/solo
controls to real flags.

**Approach.**
- Extract the two intent bodies (`apply_sign`, `apply_resume`) the dock callbacks share,
  so the loop is drivable from a test through the same code the UI runs.
- An e2e test: build a headless `StudioWindow`, drive the playback state machine to the
  High gate, `apply_sign` two enrolled roles, assert quorum via `refresh`/`AppState`,
  `apply_resume`, run to done, and verify the audit chain.
- Wire dry-run/solo to real dispatch intent (the flags already exist; assert they flow).

**Out of scope.** Drag-to-reorder + Swarm View (new tactile features, not debt); a true
mouse-event simulator (the intent-level harness gives equivalent coverage without the
Slint set-platform-once constraint).

**Plan.**

| # | Step | Status |
|---|---|---|
| 1 | Extract `apply_sign` / `apply_resume`; callbacks delegate | done |
| 2 | e2e harness: gate→sign→resume→done→audit, asserted | done |
| 3 | dry-run/solo → real dispatch | → deferred (needs canvas→CapsuleDispatch) |
| 4 | Default + core-live green, zero warnings | done |

**Daily updates.**

- 2026-06-04 — kickoff. Planset renumber: hardening took the STUDIO-7 slot, so the
  original interactivity/e2e is STUDIO-8 and packaging is STUDIO-9.
- 2026-06-04 — **e2e harness landed.** Extracted `apply_sign`/`apply_resume` (the dock
  callbacks delegate). `e2e_gate_sign_resume_audit_loop` builds a real headless
  `StudioWindow` and drives the full operator loop — play → gate pause → sign two enrolled
  roles to quorum → resume → run to done → audit verifies — asserting each transition
  through the real `refresh → AppState` bridge. Runs under **both** builds; under
  `core-live` the sign step drives the real async `ApprovalQueue`. 35 default / 39
  core-live; zero warnings.

**Decisions made.**

- **Intent-level e2e over a mouse-event simulator.** Driving the real intent code
  (`apply_sign`/`apply_resume`) + the real `refresh→AppState` bridge gives equivalent
  coverage of the gate→sign→resume→audit loop, without fighting Slint's set-platform-once
  constraint. The callbacks are now thin delegators to the tested intents.
- **dry-run/solo → real dispatch is deferred, honestly.** The flags flow to state today
  but the canvas playback is a *modeled timer*, not real `CapsuleDispatch`. Making
  dry-run/solo (and drag-reorder) affect *real* dispatch needs the canvas wired to the
  runtime's dispatcher — the same "live run from the canvas" piece the chain-writes /
  CIT-AGENT-3 work also waits on. Scoped to a future live-operation sprint, not faked here.

**Exit criteria.**

- [x] An e2e test drives gate→sign→resume→done headlessly and asserts each transition.
- [x] The audit chain verifies the completed run.
- [x] Default + core-live green (35 / 39), zero studio warnings.
- [→] dry-run/solo → *real* dispatch — deferred with the canvas→dispatch wiring (documented).

**Close note.**

STUDIO-8 closes the test-harness debt the STUDIO-1 retro flagged eight sprints ago: a
headless e2e that builds a real `StudioWindow` and drives the entire operator loop —
play → gate → sign-to-quorum → resume → done → audit — through the *same* intent code the
UI runs, asserting every transition, under both feature builds (and through the real async
`ApprovalQueue` under `core-live`). The deferred tactile features (dry-run/solo/reorder
→ *real* dispatch, the Swarm View) are honestly scoped to the future canvas→`CapsuleDispatch`
wiring — the flags flow to state, but the playback is still a modeled timer, and faking
"affects real dispatch" would be the shortcut this project doesn't take. Retro + journal +
essay alongside.
