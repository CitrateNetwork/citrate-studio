---
sprint: STUDIO-21
title: Drop the capsule bypass now that CIT-AGENT-3e signs the fleet (F-4 fully closed)
created: 2026-06-06T00:00:00Z
branch: audit/studio-21-capsule-trust
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: closed
audit_finding: F-4 (final closure), F-6 (production trust anchor)
depends_on: citrate-agent-runtime CIT-AGENT-3e (signed .cps fleet + verified dispatch)
---

# STUDIO-21 — Drop the capsule bypass (F-4 final closure)

> The runtime's **CIT-AGENT-3e** landed (packer + signed `.cps` fleet + `from_archive_verified`
> on load + the `CITRATE_ALLOW_UNVERIFIED_CAPSULES` bypass **deleted**). Studio no longer needs
> to touch that env var: the shipped fleet now loads + verifies on its own.

## Change

- **`src/core_bridge.rs` `dispatch::greet` (core-live):** removed the `set_var`/WARN/restore
  of `CITRATE_ALLOW_UNVERIFIED_CAPSULES` entirely. `greet()` now just `call_raw`s the `hello`
  capsule; the runtime loads the signed `hello.cps` through `from_archive_verified` (content_hash
  + ed25519 publisher signature + WIT cross-check) with no override. The STUDIO-18 residual
  (process-global env race) is gone because the env mutation is gone.

## F-6 note

The production capsule trust anchor is the runtime's `bundled_key::BUNDLED_PUBLISHER_KEY`
(where dispatched capsules are actually verified). Studio's `config::PublisherTrustStore` seam
(STUDIO-18) already refuses a self-carried publisher key for the onboarding-install demo; it is
not duplicated with the bundled key here (that would be dead code — Studio dispatches via the
runtime, which owns verification). No Studio change needed for F-6 beyond STUDIO-18.

## Regression gate

- `cargo clippy --all-targets` (default) — zero warnings.
- `cargo test` (default) — **43 passed**.
- `cargo test --features core-live` — **47 passed**, incl.
  `real_hello_capsule_dispatches_through_wasmtime` (returns "Hello, Aleia" with NO bypass) and
  `canvas_run_dispatches_real_capsules` — end-to-end proof the signed fleet runs verified.
- `scripts/visual-harness.sh check` — 19/19.

## Notes

- Verified against the runtime branch `audit/capsule-3e-finish` (rebased on runtime `origin/main`,
  so it carries RM-G.1's `StaticSignerRoster`). Merge the runtime CIT-AGENT-3e PR before/with this.
- Production swaps the bundled STAGING key for the HSM key (runtime `bundled_key` + re-pack);
  no Studio change required for that rotation.
