---
created: 2026-06-05T18:30:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
sprint: citrate-studio audit remediation
status: draft
audit_ref: ../../../../handoffs/STUDIO_SECURITY_AUDIT_REPORT.md
audit_sha: 6f4d7c898ad70c3d89e100a4eb6e2a25f0218e62
---

# Planset — Citrate Studio Audit Remediation (STUDIO-15…19)

> **Purpose.** The independent Tier-1 review of `citrate-studio` @ `6f4d7c8`
> (`handoffs/STUDIO_SECURITY_AUDIT_REPORT.md`) returned **PASS WITH CONDITIONS**: the
> architecture is sound and the `core-live` path holds, but **five Medium** + two Low/Info
> conditions must be closed (or accepted with accurate documentation) before the written
> `v1.0.0` attestation. This planset sequences exactly those fixes — **and nothing else** —
> with a regression-safety contract on every sprint. It is the bridge from "hardened
> release candidate" to "attestable against a SHA."

## Definition of "done"

The remediation is complete when **all of the following hold at a single new SHA**:

1. Every audit finding F-1…F-7 is either **fixed** or **explicitly risk-accepted** with
   documentation the auditor's "do not misrepresent the risk" bar would pass.
2. The two over-claims the audit caught are corrected in the source-of-truth docs
   (the "exactly 7 unwraps" count → 9; the "loudly-logged" capsule opt-in claim).
3. **No regression.** Every guard in the Regression-safety contract (below) is green at the
   close of every sprint, not just at the end.
4. The trust boundary is **tighter, not looser**: enforcement only ever moves *out of* the
   view layer *into* Rust — never the reverse — and no `.slint` file gains a policy decision.

When all four hold, Studio re-enters the audit gate (`AUDIT_TIER.md`) for the attestation
pass against the new SHA.

## Findings → sprints map

| Finding | Sev | Sprint | One-line fix |
|---|---|---|---|
| **F-1** Default-build SoD/dedup/self-approval live in the UI layer, not the policy seam | Med | **STUDIO-15** | Make `apply_sign`/`apply_resume` structurally enforce the `add_signature`-class rules; parity-test the *seam*, not just the UI helper |
| **F-2** §3.1.3.7 JWKS-skip soundness rests on TLS, but `CITRATE_STUDIO_ISSUER` can downgrade to `http://` unchecked | Med | **STUDIO-16** | Assert `https://` issuer (narrow loud localhost dev exception); reject empty issuer; `aud` membership + `azp` |
| **F-3** Default first-run persists 5 plaintext ed25519 private keys to `roster.json` | Med | **STUDIO-17** | Default first-run to Keyring; never write a `secret` to disk in a shipping build; insecure-storage banner |
| **F-5** `CITRATE_STUDIO_SIGNEDIN`/`_KYC` fake the session in the shipping binary | Low | **STUDIO-17** | Gate dev seeds behind `debug_assertions` / a dev feature |
| **F-4** `CITRATE_ALLOW_UNVERIFIED_CAPSULES` set silently + process-globally (docs say "loudly logged") | Med (core-live) | **STUDIO-18** | Per-dispatch (non-sticky) opt-in + `WARN` log + UI banner |
| **F-6** `verify_capsule` trusts the publisher key carried with the capsule (no pinning) | Low/Info | **STUDIO-18** | Add a pinned-publisher trust-store seam ahead of the real `.cps` registry |
| **F-7** Documented `cargo deny`/`audit` merge-gate is not wired into CI; `deny.toml` out of sync | Med | **STUDIO-19** | Wire the gate into CI; reconcile `deny.toml` with the graph; correct the two doc over-claims |

Severity and ordering follow the audit: highest blast-radius / trust-boundary work
(STUDIO-15) lands first, under the most scrutiny; CI + documentation cleanup lands last.

## Regression-safety contract (applies to EVERY sprint)

Each sprint closes only when **all** of these are green — this is the gate, not a wish:

- **Both builds green, zero new warnings.** `cargo build` *and* `cargo build --features core-live`; `cargo clippy --all-targets` clean.
- **Test count never drops.** `cargo test` ≥ 35 (default) and `cargo test --features core-live` ≥ 40. Fixes **add** tests; they do not delete or weaken existing assertions. A changed assertion must be justified in the sprint's close note as a deliberate tightening.
- **The two invariant suites pass under both feature builds, unchanged in intent:**
  - the **parity suite** (`quorum_satisfaction_matches_the_runtime`, `policy_rules_match_the_runtime`, `audit_verify_clean_and_tampered`) — the seam still equals the core for the quorum tally;
  - the **e2e loop** (`e2e_gate_sign_resume_audit_loop`) — gate → sign → resume → done → audit still drives clean through the real intent code.
- **Trust boundary intact.** No `.slint` file gains a policy decision. SoD/quorum/approval enforcement only moves *into* Rust. Confirm with a diff review of `ui/**`.
- **Visual regression.** `bash scripts/visual-harness.sh check` — 19 surfaces vs golden. Any intentional pixel change (e.g. a new insecure-storage banner) is a **deliberate** baseline update, called out in the sprint close note with the before/after, never a silent drift.
- **Supply chain clean.** `cargo audit --ignore RUSTSEC-2025-0055` reports 0 vulnerabilities.
- **One reviewable commit per fix**, each carrying the failing-before / passing-after test that proves the finding is closed.

A sprint that cannot meet a guard does **not** close — it either fixes the regression or
converts the item to an explicit, documented risk acceptance in `02_OPEN_QUESTIONS.md`.

## What this planset does NOT cover

- **New features or scope.** This is remediation only. No new capability, no UX work.
- **Upstream ship gates** already named in `COMPLETION_STATUS.md`: the signed `.cps` packer
  (CIT-AGENT-3e), the L0 agent loop (CIT-AGENT-3), notarized-installer certs. F-4/F-6 here
  prepare Studio's *side* of the capsule-trust story; the registry itself remains upstream.
- **Re-litigating documented deferrals.** The audit accepted them; we only fix where the
  *code* or *docs* misrepresented the risk (F-3, F-4, and the two over-claims).
- **The `core-live`-only enforcement audit residual** (the body of `ApprovalQueue::add_signature`,
  RM-G.1): that lives in `citrate-agent-runtime`, not here. STUDIO-15 makes the *default
  build* match what the audit verified the core does; it does not re-audit the core.

## Versioning

- `v0.4.x` (current) — `6f4d7c8`, the audited release candidate.
- **`v0.4.x+1` — STUDIO-15…18**: the five Medium/Low code conditions closed, no regression.
- **`v1.0.0-rc` — STUDIO-19**: CI gate wired, docs reconciled; submit to the Tier-1 audit.
- **`v1.0.0`** — on the external attestation against the STUDIO-19 SHA (`AUDIT_TIER.md`).

## Reading order

| # | File | Covers |
|---|---|---|
| 00 | [`00_PLANSET.md`](00_PLANSET.md) | Index, "done" definition, regression contract (this file) |
| 01 | [`01_SPRINT_SEQUENCE.md`](01_SPRINT_SEQUENCE.md) | STUDIO-15…19: goal / approach / files / acceptance / regression guard |
| 02 | [`02_OPEN_QUESTIONS.md`](02_OPEN_QUESTIONS.md) | Decisions to confirm before/while building |
