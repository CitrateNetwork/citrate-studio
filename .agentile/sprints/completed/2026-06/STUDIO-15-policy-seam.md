---
sprint: STUDIO-15
title: Structural policy enforcement in the default build (audit F-1)
created: 2026-06-05T19:10:00Z
branch: audit/studio-15-policy-seam
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: closed
planset: ../../../planset/2026-06-05-studio-audit-remediation/
audit_finding: F-1 (Medium)
audit_ref: ../../../../../handoffs/STUDIO_SECURITY_AUDIT_REPORT.md
---

# STUDIO-15 — Structural policy enforcement in the default build

> Closes audit finding **F-1 (Medium)**: in the shipped default build,
> separation-of-duties / no-double-sign / no-self-approval were enforced by the UI render
> layer (`approval_roster`'s `disabled` flags + Slint button gating), not by the policy
> seam. `policy::quorum_satisfied` mirrored only the core's `satisfied_by` *tally*, not its
> `add_signature` admission gate — so the seam would accept a CO+SO pair or a double-sign
> that the real `ApprovalQueue` rejects.

## Spec

Make the **policy seam** (Rust), not the view layer, the thing that rejects an inadmissible
signature in the default build — matching what `core-live`'s real `ApprovalQueue::add_signature`
already enforces. The trust-boundary claim ("the front end never makes a policy decision")
must become true for the binary users actually run. No `.slint` policy change; the UI is
unchanged in behaviour for the legitimate path.

## Changes

- **`src/main.rs` — new `admit_signature(&RunState, role) -> Result<(), String>`**
  (`#[cfg(not(feature = "core-live"))]`): the default-build mirror of `add_signature`'s
  admission rules — fail-closed on no-enrolled-signer, reject proposer (Operator)
  self-approval, reject non-approving roles (`!policy::can_approve`), reject double-signing,
  reject SoD conflicts (`policy::is_conflict` vs every already-signed role). Error strings
  match `approval_roster`'s reasons so the UI message is identical however a signature is
  submitted.
- **`apply_sign`** (default path) now calls `admit_signature` and only signs/pushes on `Ok`;
  a rejection sets `sign_error` exactly as the `core-live` rejection path does. The
  `core-live` path (`live_sign` → real queue) is unchanged.
- **`apply_resume`** is now fail-closed on the core's decision: it no-ops when a gate is
  pending and `quorum_met()` is false, so no caller can advance a gate the core (mirror or
  real) has not approved. Previously it advanced unconditionally and relied on the UI hiding
  the button.
- **`ui/components/approval.slint`** header comment corrected to state that SoD / dedup /
  no-self-approval are enforced in Rust (the real core under `core-live`; `admit_signature`
  in the default build), never by the view layer. (Comment only — no pixel change.)
- Opportunistic clippy cleanup of pre-existing `1.94.0` lints surfaced by the build
  (`src/data.rs` `#[allow(too_many_arguments)]` on the demo `capsule()` ctor;
  `src/auth.rs` `split_once`; `src/main.rs` `contains`/`as_ref`).

## Tests (added)

- `default_build_seam_rejects_sod_double_sign_and_self_approval` — drives the real
  `apply_sign` intent: SO admitted; CO+SO rejected (`SoD…`); double-sign rejected
  (`Already signed`); Operator self-approval rejected (`Proposer…`); Auditor rejected; a
  non-conflicting Reviewer then reaches quorum. Proves the *seam* refuses, not the button.
- `apply_resume_is_a_noop_until_quorum_met` — resume does not advance the gate without
  quorum, and does once quorum is met.

## Regression gate (per the planset contract)

- `cargo build` — clean. `cargo clippy --all-targets` — **zero warnings**.
- `cargo test` — **37 passed / 0 failed** (was 35; +2, none removed/weakened).
- Invariant suites unchanged in intent and green: `quorum_satisfaction_matches_the_runtime`,
  `policy_rules_match_the_runtime`, `audit_verify_clean_and_tampered`,
  `e2e_gate_sign_resume_audit_loop`, `quorum_met_needs_two_signatures_for_high`,
  `approval_roster_enforces_separation_of_duties`.
- `scripts/visual-harness.sh check` — **19/19 surfaces match baseline** (no UI regression;
  the slint edit was comment-only).
- `cargo audit --ignore RUSTSEC-2025-0055` — unchanged (no dep change).

## Residual / follow-ups

- **`core-live` compile not run here** (SSH-gated dep — same residual the audit noted). The
  change is cfg-isolated: `admit_signature` is `not(core-live)`; `apply_resume`'s gate uses
  the feature-agnostic `quorum_met()` that the existing `dock_routes_through_the_real_live_queue`
  and `e2e_gate_sign_resume_audit_loop` tests already exercise under `core-live`. **Action:**
  run `cargo test --features core-live` (≥40) in a network-connected environment to confirm
  before the attestation.
- The beginner-chat `on_chat_approve` path (a scripted demo card, not the gate dock) still
  clears `pending` cosmetically without quorum; out of F-1 scope (it does not advance a real
  run's gate) but noted for a future tidy.
