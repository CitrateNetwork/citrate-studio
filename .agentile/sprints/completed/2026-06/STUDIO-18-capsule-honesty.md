---
sprint: STUDIO-18
title: Loud, non-sticky capsule opt-in + publisher pinning (audit F-4, F-6)
created: 2026-06-05T20:35:00Z
branch: audit/studio-18-capsule-honesty
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: closed
planset: ../../../planset/2026-06-05-studio-audit-remediation/
audit_finding: F-4 (Medium, core-live), F-6 (Low/Info)
---

# STUDIO-18 — Loud, non-sticky capsule opt-in + publisher pinning

> Closes **F-4 (Medium, core-live)** — `CITRATE_ALLOW_UNVERIFIED_CAPSULES` was set silently
> and process-globally (sticky for the whole process), despite docs claiming a "loudly-logged"
> opt-in — and **F-6 (Low/Info)** — `verify_capsule` trusted the publisher key carried with the
> capsule, so any self-signed `(bytes, sig, pubkey)` triple verified (integrity, not authenticity).

## Spec (decisions locked — planset 02: Q4)

- **F-4:** the core (`citrate_agent_core::capsule::dispatch`) reads the bypass var from the env
  only (no per-call flag; checked at `call_raw` time). So: emit a `WARN`, set the var, run, and
  **restore the prior value after the call** — never sticky.
- **F-6:** add a pinned-publisher trust store; a capsule installs only if its publisher is
  pinned AND its signature verifies.

## Changes

- **`src/core_bridge.rs` `dispatch::greet` (core-live):** `eprintln!` WARN at the bypass site;
  capture the prior `CITRATE_ALLOW_UNVERIFIED_CAPSULES`, set `=1`, dispatch, then restore
  (`set_var`/`remove_var`). The gate is no longer left open for the process lifetime.
- **`src/config.rs`:** new `PublisherTrustStore` (pinned pubkeys); `verify_capsule(c, trust)`
  requires the publisher be pinned **and** the signature verify; `install_capsules(sources,
  trust)` likewise; `demo_capsule_sources()` → `demo_capsule_sources_with_trust()` returns the
  feed + a store pinning its publisher (the rogue capsule keeps an invalid sig → still rejected).
- **`src/main.rs`** `onboard_apply` uses the new `(sources, trust)` API.

## Tests

- Added `capsule_verify_rejects_unpinned_publisher` — a valid self-signature from an unpinned
  key is rejected; pinning it makes it pass.
- Updated `capsule_install_is_fail_closed`, `capsule_verify_rejects_tamper`,
  `onboarding_builds_a_persistable_config` to the trust-store API (deliberate: authenticity is
  now required, not just integrity).

## Regression gate

- `cargo clippy --all-targets` — **zero warnings**.
- `cargo test` — **42 passed / 0 failed** (was 41; +1).
- `scripts/visual-harness.sh check` — **19/19 match** (no UI change).
- `cargo audit` — unchanged.

## Residual

- **F-4 (`core_bridge.rs`) is core-live and was NOT compiled here** (SSH-gated dep). The change
  is small and mirrors the existing call pattern; **action:** `cargo test --features core-live`
  in a connected env must confirm `real_hello_capsule_dispatches_through_wasmtime` still passes
  and that `CITRATE_ALLOW_UNVERIFIED_CAPSULES` is unset after a dispatch (the F-4 property).
- `set_var`/`remove_var` are process-global; sequential dispatch (the studio path) is safe, but
  concurrent dispatch on multiple threads could still race the gate — acceptable for a dev
  opt-in, removed entirely when CIT-AGENT-3e signs the fleet and the bypass is deleted.
- F-6's trust store is the **seam** for the real registry; CIT-AGENT-3e must populate it from a
  signed source of record. Today only the demo publisher is pinned.
