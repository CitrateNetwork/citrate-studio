---
created: 2026-06-05T18:40:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
sprint: citrate-studio audit remediation
status: draft
---

# 02 — Open Questions & Assumptions

> Decisions to confirm before/while building STUDIO-15…19. Each names the sprint it gates,
> the options, and the planset's **recommended default** so building can proceed if no other
> answer arrives. Resolved answers should be recorded inline and, if load-bearing, promoted
> to an ADR under `.agentile/adrs/`.

## ✅ Decisions locked (2026-06-05, Saul Loveman)

| Q | Decision |
|---|---|
| **Q1** first-run roster | **Seed nothing; explicit enrollment.** Clean install shows the fail-closed "No signer enrolled" state; operator enrolls each role onto Keyring/hardware deliberately. Dev demo seed available under `--features dev-filebacked`. |
| **Q2** dev issuer | **Debug builds only, with a loud `WARN`.** Release builds reject any non-`https://` issuer outright; `http://127.0.0.1`/`localhost` allowed only under `#[cfg(debug_assertions)]`. |
| **Q3** key persistence | **Dev-gate secret serialization.** Secret-to-disk compiles only under `cfg(any(test, feature = "dev-filebacked"))`; shipping serialization writes pubkey/role/id only. |
| **Q4** F-4 mechanism | **Guarded-env (resolved from source).** `citrate-agent-core::capsule::dispatch` reads `CITRATE_ALLOW_UNVERIFIED_CAPSULES` from the env only (no per-call flag; checked at `call_raw` time — `dispatch.rs:111`). STUDIO-18 sets it, `WARN`s, and **restores the prior value after the call** so it is not sticky. |
| **Q5** deny.toml | **Per-advisory ignore + `allow-git` the pinned rev.** Keeps `unmaintained = deny` strict for new advisories; the 6 transitive advisories get documented ignores mirroring `audit.toml`. |
| **Q6** risk-acceptance | Per the assumption below: F-1/F-2/F-3/F-7 are fix-before-attestation; F-4/F-6 may be documented risk-acceptances only if blocked upstream. |

## Q1 — Default first-run roster posture (gates STUDIO-17 / F-3)

What should a clean shipping install do for the approval roster on first run?

- **(a) Recommended — seed nothing; require explicit per-role enrollment.** First run shows
  the fail-closed state the UI already renders ("No signer enrolled"); the operator enrolls
  each role onto Keyring (or hardware) deliberately. Most defensible: no key material exists
  until a human chooses a surface. Cost: onboarding has one more real step (already designed —
  the roster onboarding step exists).
- (b) Seed onto **Keyring** automatically. Lower friction, no plaintext on disk — but
  auto-generated keys with no human custody still back the lattice (weaker provenance), and it
  prompts the OS keychain on first run.
- (c) Keep FileBacked seed, but only behind an explicit "insecure dev mode" the user opts into.

*Recommendation: (a), with (c) available under `--features dev-filebacked` for demos.*
*Impact if unresolved: STUDIO-17 cannot finalize `load_or_seed`/`seed_demo` shipping behaviour.*

## Q2 — localhost dev-issuer policy (gates STUDIO-16 / F-2)

Should a plaintext `http://127.0.0.1`/`localhost` issuer be allowed at all, and where?

- **(a) Recommended — allow only under `#[cfg(debug_assertions)]` (or a `dev-auth` feature),
  with a `WARN` log.** Release builds reject any non-`https` issuer, full stop. Keeps the
  loopback mock test green (it runs in the debug profile) while making the shipping binary
  https-only.
- (b) Allow localhost http in all builds (current behaviour) — rejected by the audit's logic.
- (c) Never allow http, even in dev — would require the test/dev IdP to serve TLS.

*Recommendation: (a). Confirm the mock test (`loopback_capture_and_token_exchange_against_a_mock`)
runs under the dev gate so STUDIO-16 doesn't break it.*

## Q3 — FileBacked secret persistence: remove or dev-gate? (gates STUDIO-17 / F-3)

The audit flagged `Roster::to_json` writing the FileBacked `secret` to disk.

- **(a) Recommended — dev-gate it.** Secret serialization compiles only under
  `#[cfg(any(test, feature = "dev-filebacked"))]`; shipping serialization emits pubkey/role/id
  only. Keeps tests and demos working without plaintext keys in the shipped product.
- (b) Remove FileBacked entirely — simplest security story, but breaks the no-keychain demo
  flow and several existing tests that rely on `secret.unwrap()`.

*Recommendation: (a). Pairs with Q1(a)/(c).*

## Q4 — F-4 mechanism: per-call flag vs guarded env (gates STUDIO-18)

Does the core's `CapsuleDispatch` API accept an "allow unverified" flag per call/load, or does
it only read the `CITRATE_ALLOW_UNVERIFIED_CAPSULES` env?

- If **per-call** is available: pass the flag explicitly (no process-global mutation) — cleanest.
- If **env-only**: set it, `WARN`, and **restore the prior value** after the call so it is not
  sticky; never leave the gate open for the process lifetime.

*Action: confirm the `citrate-agent-core` `CapsuleDispatch` surface before STUDIO-18; this is an
upstream API fact, not a Studio decision. Default to the guarded-env approach if per-call is absent.*

## Q5 — `deny.toml` reconciliation scope (gates STUDIO-19 / F-7)

Should the 6 transitive unmaintained advisories be `ignore`d in `deny.toml`, or should
`unmaintained` drop from `deny` to `warn`?

- **(a) Recommended — `ignore` each of the 6 with a rationale mirroring `audit.toml`,** and
  `allow-git` the pinned SSH rev. Keeps `unmaintained = deny` strict for *new* advisories while
  making the current graph pass deterministically. Most auditable.
- (b) Set `unmaintained = "warn"` — simpler, but loses the strict default and the audit trail
  per-crate.

*Recommendation: (a). The green must be real (the audit's F-7 critique), so prefer explicit,
per-advisory documented ignores over loosening the global policy.*

## Q6 — Risk-acceptance fallback

If a finding cannot be fixed within its sprint (e.g. an upstream API gap for F-4/Q4), is an
**explicit, documented risk acceptance** acceptable for the `v1.0.0` attestation, or must the
fix block the release?

- The audit's bar: a deferral is fine **only if the documentation does not misrepresent the
  risk**. Any accepted item must land here with severity, blast radius, the gating dependency,
  and the re-evaluation trigger — matching the existing "Known non-issues" discipline.

*Assumption (until told otherwise): F-1, F-2, F-3, F-7 are fix-before-attestation (they affect
the shipping build's security posture or the gate's integrity); F-4, F-6 may be documented
risk-acceptances if blocked upstream, since they are core-live / forward-looking.*

---

## Assumptions carried into the build

1. The `core-live` enforcement the audit verified by reading `quorum.rs`/`roles.rs` (and
   inferred for `add_signature`/RM-G.1) is correct; STUDIO-15 mirrors it in the default build
   rather than re-auditing the core (that residual lives in `citrate-agent-runtime`).
2. No `.slint` file should change for policy; the only intentional UI changes are the two
   insecure-state banners (F-3, F-4), each a deliberate visual-baseline update.
3. The existing test injection pattern (`new_state_with(seed_demo())`) remains the test seam,
   so dev-gating shipping behaviour does not reduce coverage.
4. Sprint estimates assume no upstream blocker on Q4; if `CapsuleDispatch` needs an API change,
   STUDIO-18 converts to a documented risk-acceptance + an upstream ticket.
