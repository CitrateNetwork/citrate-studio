---
sprint: STUDIO-19
title: Wire the supply-chain gate into CI + reconcile docs (audit F-7)
created: 2026-06-05T21:20:00Z
branch: audit/studio-19-supply-chain-ci
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: closed
planset: ../../../planset/2026-06-05-studio-audit-remediation/
audit_finding: F-7 (Medium) + doc corrections
---

# STUDIO-19 — Wire the supply-chain gate into CI + reconcile docs

> Closes **F-7 (Medium)** — `deny.toml`/`audit.toml` documented a merge-blocking supply-chain
> gate that **no CI workflow ran**, and `deny.toml` was out of sync with the graph — plus the
> two over-claims the audit caught ("7 unwraps" → 9; "loudly-logged" capsule opt-in).

## Spec (decisions locked — planset 02: Q5; + a new license decision)

- Wire `cargo audit` + `cargo deny` into CI on PR + push.
- Reconcile `deny.toml` so `cargo deny check` passes **for real** (Q5: per-advisory ignore +
  `allow-git` the pinned SSH rev).
- Correct the docs.

## Changes

- **`.github/workflows/supply-chain.yml`** (new): a `cargo-audit` job (reads `Cargo.lock`, no
  source fetch — the always-on advisory gate) and a `cargo-deny` job (configures the
  `github-citrate-chain` SSH alias from `secrets.CITRATE_CHAIN_DEPLOY_KEY` so `cargo metadata`
  can resolve the optional core-live source, then runs `cargo deny check`).
- **`deny.toml`** — made it pass against the real graph **and** the current cargo-deny schema:
  - advisories: schema fix (`unmaintained = "all"`, dropped removed `notice` key); ignore the
    transitive unmaintained advisories with rationale.
  - sources: `allow-git` the pinned `ssh://git@github-citrate-chain/CitrateNetwork/citrate-chain`.
  - bans: `Cargo.toml` `publish = false` (app + private UI kit) so the `ui-kit` path dep isn't a
    wildcard violation.
  - licenses: added the licenses actually present in the graph — `BUSL-1.1` (first-party),
    `BSL-1.0` (Boost), `CDLA-Permissive-2.0`, `NCSA`, and **`LicenseRef-Slint-Software-3.0`**.
- **`COMPLETION_STATUS.md`** — "7 non-test unwraps" → **9 (5 default + 4 core-live)**, per audit §6.
- **`.agentile/audits/2026-06-05-studio-security/REMEDIATION.md`** — finding→commit→test evidence.

## Decision recorded — Slint license basis

Running cargo-deny for real surfaced that Slint is tri-licensed
(`GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0`). GPL-3.0
is incompatible with the BUSL-1.1 product, so the project ships Slint under **Slint Software
3.0** (operator decision, 2026-06-05). `deny.toml` allows `LicenseRef-Slint-Software-3.0`; GPL
is intentionally NOT allowed.

## Regression gate

- **`cargo deny check` — advisories ok, bans ok, licenses ok, sources ok** (verified locally;
  cargo-deny was installed for this sprint — it was absent at audit time, the root of F-7).
- `cargo audit --ignore RUSTSEC-2025-0055` — 0 vulnerabilities.
- `cargo clippy --all-targets` — clean. `cargo test` — **42 passed**. Visual — **19/19**.

## Notes / residual

- The audit DB drifted since the report: a few ignored advisory IDs are now "not encountered"
  (warnings, non-failing) — kept as a documented superset so the gate stays green across DB
  versions.
- The `cargo-deny` CI job needs `secrets.CITRATE_CHAIN_DEPLOY_KEY` (the core-live SSH source);
  `cargo-audit` does not. Confirm the secret exists, then verify the workflow on a real PR.
- License findings (BSL/CDLA/NCSA/Slint) were pre-existing facts the gate never caught because
  it was never run — now they are explicit and reviewed, which is the point of F-7.
