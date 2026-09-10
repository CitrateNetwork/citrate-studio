---
created: 2026-06-04T21:00:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-1
---

# ADR-2026-06-04 — Slint version, renderer, and font strategy

## Status

Accepted.

## Context

citrate-studio is a new native Slint app and the intended UI-kit base for the other
Citrate native shells. Three foundational choices had to be made up front: which Slint
version, which renderer/backend, and how to load the four brand fonts.

- The federation's existing Slint apps (`gui-native`, a Fortune-200 aerospace partner shell,
  `learning-center`) pin **Slint 1.9** with `renderer-software` + `backend-winit` +
  `compat-1-2`.
- The latest stable Slint is **1.16.1**.
- The shell must render headlessly in CI / a windowless shell for fidelity review.
- The brand uses Geist, Geist Mono, Space Grotesk, Cormorant.

## Decision

1. **Slint 1.16.1**, not the federation's 1.9. This kit is the forward-looking base;
   1.16 is backward-compatible via `compat-1-2`, and starting on the latest avoids a
   migration debt the moment the kit is extracted for reuse.
2. **`renderer-software` + `backend-winit`.** Matches the federation siblings,
   pure-CPU (no GPU / `skia` build dependency — "no weird deps"), and — decisively —
   it renders headlessly via `MinimalSoftwareWindow` for snapshot-based fidelity
   review. `renderer-femtovg` (GPU) and `renderer-skia` (max text fidelity) remain a
   one-flag swap in `Cargo.toml` for a shipped desktop build if profiling warrants.
3. **Compile-time font embedding** via `import "*.ttf"` at the top of `studio.slint`
   (Slint's recommended path), not runtime registration — which in 1.16 moved behind
   the unstable `unstable-fontique-08` feature.

## Consequences

- Headless screenshots work without a window surface — the core fidelity loop.
- The kit is on the latest Slint; when extracted into a shared crate the other shells
  upgrade *to* it rather than the kit being held back to 1.9.
- femtovg's `take_snapshot()` is unreliable headlessly (no presented surface); the
  software `MinimalSoftwareWindow` path is the canonical capture method. Documented in
  `docs/journals/2026-06-04T2100_the-blank-window.md`.
- A future federation-wide Slint version bump should align the other shells to 1.16+.
