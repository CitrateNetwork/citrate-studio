---
sprint: STUDIO-20
title: Post-remediation cleanup — the "future tidy" follow-ups (F-1/F-2/F-4 residuals)
created: 2026-06-06T00:00:00Z
branch: audit/studio-20-cleanup
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: closed
planset: ../../../planset/2026-06-05-studio-audit-remediation/
audit_finding: residual tidies flagged in STUDIO-15 / STUDIO-16 / STUDIO-18
---

# STUDIO-20 — Post-remediation cleanup

> Closes the three "future tidy" follow-ups the remediation sprints flagged in their residual
> sections, now that DevOps has verified core-live + CI (F-1…F-7 all ✅, see
> `.agentile/audits/2026-06-05-studio-security/REMEDIATION.md`).

## Changes

- **STUDIO-15 residual — beginner-chat `on_chat_approve` fabricated an approval.** It cleared
  `pending` and narrated "the core returned Approved" with no core decision. Now it **routes to
  the real, policy-seam-gated dock** (keeps the gate pending, switches to the advanced
  workspace, sets `paused`), so the operator signs 2-of-N in the STUDIO-15-hardened dock and the
  core decides. No fabricated approval, no fake completion card. (`src/main.rs` `on_chat_approve`;
  the now-unused `cstep`/`CardStep` helper removed.)
- **STUDIO-16 residual — `logout()` was not issuer-guarded.** It now runs the network revoke
  only when `validate_issuer()` passes (never POSTs a bearer token to a plaintext issuer), and
  **always clears local tokens** so logout still succeeds locally on a bad issuer.
  (`src/auth.rs` `logout`.)
- **STUDIO-18 residual — capsule env-var race** (process-global `set_var`/`remove_var`): no code
  change here. The non-sticky assertion was added during DevOps core-live verification (F-4 ✅).
  The residual concurrency race is inherent to the env-var bypass and is **closed when the
  bypass is deleted** by the CIT-AGENT-3e `.cps` signing work — tracked there, not a Studio bug.

## Tests

- Added `logout_skips_network_on_insecure_issuer_but_clears_local` — a plaintext non-loopback
  issuer gets no bearer POST, but local tokens are cleared and logout returns `Ok`.
- `on_chat_approve` is a UI event handler (no fabricated state to assert); covered by build +
  the visual gate (no initial-render change). The dock it routes to is covered by the existing
  STUDIO-15 seam tests + `e2e_gate_sign_resume_audit_loop`.

## Regression gate

- `cargo clippy --all-targets` — **zero warnings** (removed the now-dead `cstep`).
- `cargo test` — **43 passed / 0 failed** (was 42 default; +1).
- `scripts/visual-harness.sh check` — **19/19 match baseline** (no UI render change).
- `cargo audit` / `cargo deny check` — unchanged (no dep/manifest change).

## Notes

- This is the last *Studio-internal* remediation item. What remains for `v1.0.0` is cross-repo /
  infra: the CIT-AGENT-3e `.cps` packer (deletes the F-4 bypass + populates the F-6 trust store),
  the L0 agent loop (CIT-AGENT-3), notarized installers (Apple/Windows certs), and the external
  Tier-1 attestation. See the planset + `COMPLETION_STATUS.md`.
