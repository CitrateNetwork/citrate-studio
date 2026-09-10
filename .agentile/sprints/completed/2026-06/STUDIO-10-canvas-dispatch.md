---
created: 2026-06-04T07:10:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: archived
sprint: STUDIO-10
---

# STUDIO-10 — Canvas drives real CapsuleDispatch

**Goal.** The Composition Canvas's playback triggers **real** capsule execution: as each
clip completes during a run, a real capsule dispatches through wasmtime (`core-live`) and
the output card shows a real-execution provenance badge. Closes the last non-gated
functional item in `COMPLETION_STATUS.md` (the canvas live-run).

**Honest design.** The demo scenario clips (`recon.*`) have no matching capsule in the
fleet yet (those land with the upstream packer), so the run dispatches the real `hello`
smoke capsule per clip — proving the canvas *drives* real `CapsuleDispatch` with a real
`ToolResult` — and the output card keeps the modeled scenario result **plus** a "⚡ wasmtime"
provenance line carrying the real return. Real execution, honestly labelled: no claim that
the recon output itself is real.

**Plan.**

| # | Step | Status |
|---|---|---|
| 1 | `RunState`: `just_completed` / `outputs` / `dispatch` (core-live) | done |
| 2 | `tick` records completions; `dispatch_completions` runs real capsules | done |
| 3 | `ClipData.live-output` → OutputCard provenance badge | done |
| 4 | e2e: a run dispatches real capsules; outputs populated (core-live) | done |
| 5 | Default + core-live green; screenshot | done |

**Daily updates.**

- 2026-06-04 — kickoff.
- 2026-06-05 — **shipped.** The canvas drives real dispatch: `tick` records every clip
  finishing a tick; `dispatch_completions` runs the real capsule (wasmtime) per completion
  and stores the `ToolResult`; the output card shows a "⚡ wasmtime" provenance badge.
  `apply_seed` now drives the real state machine + dispatch (honest seeds). Screenshot
  confirms `wasmtime · hello("recon.snapshot") → "Hello, recon.snapshot"` on the cards.
  `canvas_run_dispatches_real_capsules` (core-live) asserts outputs populate. 35 / 40 tests,
  both builds zero warnings.

**Decisions made.**

- **Dispatch the real `hello` smoke, label it precisely.** The scenario's `recon.*` capsules
  don't exist yet (upstream packer), so the card keeps the modeled scenario result and adds
  a badge that claims only what's true: a real capsule ran via wasmtime.
- **Pure `tick`, isolated I/O.** `tick` records completions; `dispatch_completions` does the
  dispatch — the state machine stays pure + testable.

**Exit criteria.**

- [x] Under `core-live`, a run dispatches a real capsule per completed clip (wasmtime).
- [x] Output cards show the real-dispatch provenance; default build unchanged.
- [x] `canvas_run_dispatches_real_capsules` asserts the live-run populates real outputs.
- [x] Default + core-live green (35 / 40), zero warnings.

**Close note.**

STUDIO-10 closes the last non-infra functional item: the Composition Canvas now drives real
`CapsuleDispatch`. Each clip completion triggers a real wasmtime execution and the output
card carries a "⚡ wasmtime" provenance badge with the real `ToolResult` — labelled to the
exact resolution of its realness (a real capsule ran; not the not-yet-built recon capsule,
so the modeled scenario result stays and the badge claims only the real execution). The
remaining live items are genuinely gated: real *scenario* capsules (upstream packer),
off-thread dispatch for heavy capsules, and chain anchoring (a funded key). Retro + journal
+ essay alongside.
