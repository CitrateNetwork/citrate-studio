---
created: 2026-06-05T18:35:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
sprint: citrate-studio audit remediation
status: draft
---

# 01 — Sprint Sequence (STUDIO-15…19)

> Five focused sprints, one per finding-cluster, ordered by blast radius. Each carries the
> Regression-safety contract from `00_PLANSET.md` as its non-negotiable close gate — it is
> restated per sprint only where a sprint adds a *specific* extra guard. Estimates are
> engineering-days for a focused operator+agent pair.

---

## Phase A — Trust boundary (the headline)

### STUDIO-15 — Structural policy enforcement in the default build  · ~2–3 days · **F-1**

**Goal.** The shipped (default) build's policy *seam* — not its UI render layer — enforces
separation-of-duties, no-double-sign, no-self-approval, and Auditor-exclusion, matching what
the real `ApprovalQueue::add_signature` enforces under `core-live`. "The front end never
makes a policy decision" becomes true for the binary users actually run.

**Why first.** Highest-severity trust-boundary finding; touches the approval flow, so it is
also the most regression-sensitive. Landing it first gives it maximum review time and forces
the parity/e2e guards to prove the change is behaviour-preserving for the legitimate path.

**Approach.**
- Introduce a single chokepoint the default build calls before a signature is counted —
  e.g. `fn admit_signature(state, role) -> Result<(), SignError>` — that mirrors the core's
  `add_signature` admission rules using the existing `policy::is_conflict` / `policy::can_approve`
  predicates plus same-role dedup and proposer self-approval rejection. `apply_sign`
  (`src/main.rs:443`) calls it and only `push`es on `Ok`; a rejection sets `sign_error`
  exactly as the `core-live` rejection path already does (so the UI is identical).
- `apply_resume` (`src/main.rs:504`) becomes a **no-op unless `quorum_met()`** — the gate
  cannot advance without the core's (mirrored) decision, regardless of what invoked it.
- `approval_roster()` keeps computing `disabled`/`reason` for display, but those flags are now
  a *rendering* of the chokepoint's verdict, not the only thing enforcing it. The UI is
  unchanged; the enforcement is relocated, not duplicated.
- Keep the chokepoint logic in the `policy`/seam layer so `core-live` continues to route
  through the real queue and the default build routes through the mirror — same call shape.

**Files.** `src/main.rs` (`apply_sign`, `apply_resume`, the new admission fn, `approval_roster`);
tests in `src/main.rs` `mod tests`. **No `.slint` change** (verify).

**Acceptance.**
- [ ] New seam-level tests prove the **default build** rejects, at the policy layer (not via
      the UI `disabled` flag): a CO+SO pair on a High gate; the same role signing twice; the
      proposer (Operator) self-approving; an Auditor signature.
- [ ] `apply_resume` is proven a no-op while `!quorum_met()` and only advances when met.
- [ ] The existing `quorum_satisfaction_matches_the_runtime` is **extended** to assert the
      seam (not just `satisfied_by`) rejects the SoD pair and the double-sign — under **both**
      feature builds.
- [ ] `approval.slint:7` comment ("SoD is computed by the core") is now accurate for both
      builds; update the wording if needed to say where (seam) it is computed.
- [ ] Regression contract green — **especially** `e2e_gate_sign_resume_audit_loop` and
      `dock_routes_through_the_real_live_queue` unchanged in intent.

**Regression risk + guard.** This rewrites the signature-admission path. Guard: the e2e loop
and the legitimate two-signature High-gate flow (`quorum_met_needs_two_signatures_for_high`,
`approval_roster_enforces_separation_of_duties`) must pass byte-for-byte in outcome; add the
new rejection tests *alongside* them, never replacing them.

---

## Phase B — Auth hardening

### STUDIO-16 — Enforce the TLS precondition + tighten claim validation  · ~1–2 days · **F-2**

**Goal.** The OIDC §3.1.3.7 JWKS-skip is only sound over authenticated TLS; make the code
*enforce* the precondition it depends on, so the skip cannot silently become unauthenticated
trust over plaintext.

**Approach.**
- In `auth_config()` (`src/main.rs:958`) / `AuthConfig`, **reject a non-`https://` issuer**,
  with one narrow exception: an explicit `http://127.0.0.1[:port]` / `http://localhost[:port]`
  dev issuer, allowed **only** behind `#[cfg(debug_assertions)]` (or a `dev-auth` feature) and
  emitted as a `WARN`-level log ("INSECURE: plaintext issuer — dev only"). A release build
  with an `http` issuer fails closed (`AuthError`), never silently downgrades.
- In `validate_claims` (`src/auth.rs:163`): **reject an empty `cfg.issuer`** rather than
  skipping the issuer check; keep `exp`/`wallet_address` checks.
- In `decode_claims` (`src/auth.rs:144`): for an array `aud`, verify the client_id is **a
  member** of the array (not just the first element), and require `azp == client_id` when the
  array has more than one audience.

**Files.** `src/auth.rs` (`AuthConfig`, `decode_claims`, `validate_claims`), `src/main.rs`
(`auth_config`); tests in `src/auth.rs` `mod tests`.

**Acceptance.**
- [ ] A release-config `http://` issuer is rejected (test); a debug `http://127.0.0.1` issuer
      is allowed with a logged warning (test under `debug_assertions`).
- [ ] Empty-issuer config is rejected by `validate_claims` (test).
- [ ] `aud` as `["other","citrate-studio"]` validates; `["other"]` rejects; multi-aud without
      matching `azp` rejects (tests).
- [ ] Existing auth tests unchanged: `pkce_s256_rfc7636_vector`, `decode_and_validate_claims`,
      `token_response_parses`, `loopback_capture_and_token_exchange_against_a_mock` still pass.
- [ ] `COMPLETION_STATUS.md` §1 note on §3.1.3.7 updated to state the enforced https
      precondition.

**Regression risk + guard.** The loopback mock test uses an `http://127.0.0.1` issuer — it
must remain valid under the dev exception (run the auth tests in the debug profile, which is
the default for `cargo test`). Confirm the production guard does not break the test issuer.

---

## Phase C — Key storage + dev-seed posture

### STUDIO-17 — Production key storage by default; gate dev seeds  · ~2–3 days · **F-3, F-5**

**Goal.** The shipped binary never writes a private key to plaintext disk, and never
fabricates an authenticated session from an env var. The default first-run posture is
Keyring-backed, not FileBacked.

**Approach (F-3).**
- First-run enrollment (`signing::load_or_seed` → `seed_demo`, `src/signing.rs:262,301`):
  in a **shipping build**, do not auto-seed a working FileBacked roster. Options to confirm
  in `02_OPEN_QUESTIONS.md` Q1 — preferred: seed onto **Keyring**, or seed **no working
  keys** and require explicit per-role enrollment, surfacing the fail-closed state the UI
  already renders.
- `Roster::to_json` (`src/signing.rs:203`): never serialize a `secret` in a shipping build —
  FileBacked secret-on-disk persistence is `#[cfg(any(test, feature = "dev-filebacked"))]`
  only. Keyring secrets already live in the OS keyring; the JSON keeps only pubkey/role/id.
- Add an **insecure-storage banner** in Settings → Signer Roster whenever any enrolled signer
  is FileBacked (a deliberate, baseline-updated visual change — see guard below).
- `seed_demo` is retained for tests (tests inject a roster via `new_state_with`), so the test
  suite is unaffected.

**Approach (F-5).**
- `CITRATE_STUDIO_SIGNEDIN` / `CITRATE_STUDIO_KYC` handling in `restore_session`
  (`src/main.rs:1091`) and `headless_shot` (`src/main.rs:1831`) is gated behind
  `#[cfg(debug_assertions)]` (the headless-shot path is dev tooling and may stay under the
  same gate); a release build ignores them.

**Files.** `src/signing.rs` (`seed_demo`, `load_or_seed`, `to_json`/`from_json`), `src/main.rs`
(`new_state`, `restore_session`, `headless_shot`), `ui/components/settings.slint` (banner only);
tests in `src/signing.rs` and `src/main.rs`.

**Acceptance.**
- [ ] A release build's first run does **not** write any `secret` hex to `roster.json` (test
      the shipping serialization path; assert no secret field).
- [ ] FileBacked secret persistence compiles only under test/dev cfg; the production roster
      round-trips pubkey/role/id without secrets.
- [ ] A release build ignores `CITRATE_STUDIO_SIGNEDIN`/`_KYC` (test under non-debug cfg or
      document the cfg boundary if it can't be unit-tested).
- [ ] The insecure-storage banner renders iff a FileBacked signer exists; the visual baseline
      for the affected Settings surface is updated deliberately with before/after in the close note.
- [ ] Regression contract green; `roster_persist_roundtrip`, `save_load_to_temp_file`,
      `enroll_sign_and_fail_closed` still pass (adjusting only the secret-on-disk assertion to
      the dev cfg, justified as a deliberate tightening).

**Regression risk + guard.** Changing roster serialization can break persistence round-trips
and the seeded-signer e2e path. Guard: keep `seed_demo` + injected-roster tests intact; the
e2e loop uses `new_state_with(seed_demo())`, so it must still reach quorum. The visual harness
will flag the new banner — update that one baseline intentionally, leave the other 18 untouched.

---

## Phase D — Capsule integrity honesty

### STUDIO-18 — Loud, non-sticky capsule opt-in + publisher-pinning seam  · ~1–2 days · **F-4, F-6**

**Goal.** The capsule-integrity bypass is loud and scoped (matching the docs), and Studio has
a real seam for pinning publisher keys ahead of the upstream `.cps` registry.

**Approach (F-4, core-live).**
- Replace the sticky process-global `std::env::set_var("CITRATE_ALLOW_UNVERIFIED_CAPSULES", …)`
  in `dispatch::greet` (`src/core_bridge.rs:440`): prefer passing the unverified-allowed flag
  into `CapsuleDispatch` per-call/per-load (no process-global mutation), or — if the core API
  requires the env — set it, **emit a `WARN` log at the site** ("INTEGRITY GATE BYPASSED: running
  unverified capsule — dev only"), and **restore the prior value** after the call so it is not
  sticky for the process lifetime.
- Surface a UI banner in the run/output surface while unverified dispatch is active, so the
  operator cannot miss it (deliberate baseline update if it appears in a captured surface).

**Approach (F-6).**
- Add a `PublisherTrustStore` seam to `config::verify_capsule` (`src/config.rs:167`): a capsule
  verifies only if `c.publisher` is in a pinned allowlist *and* the signature checks out — not
  merely if the carried key self-signs. Today's `demo_capsule_sources` registers its generated
  key into the store so the demo still passes; the seam is where the real CIT-AGENT-3e registry
  feed plugs in. Document that the trust anchor (not just signature consistency) is the security
  property.

**Files.** `src/core_bridge.rs` (`dispatch`), `src/config.rs` (`verify_capsule`,
`CapsuleSource`/trust store, `demo_capsule_sources`), optionally a run-surface `.slint` banner;
tests in `src/config.rs` and `src/core_bridge.rs`.

**Acceptance.**
- [ ] `CITRATE_ALLOW_UNVERIFIED_CAPSULES` is no longer left set after a dispatch (test the
      env state before/after, core-live), and a `WARN` is emitted at the bypass site.
- [ ] `verify_capsule` rejects a validly-self-signed capsule whose publisher is **not** pinned;
      accepts one whose publisher is pinned (tests); `capsule_install_is_fail_closed` and
      `capsule_verify_rejects_tamper` still pass with the demo key pinned.
- [ ] The core-live `real_hello_capsule_dispatches_through_wasmtime` test still passes.
- [ ] `core_bridge.rs:438` comment and the journal/`COMPLETION_STATUS.md` wording now match the
      code (loud + non-sticky), or are corrected in STUDIO-19.

**Regression risk + guard.** core-live only; default build unaffected. Guard: the core-live
dispatch + doctor tests must still pass; do not change the `hello` capsule path's behaviour,
only its loudness/scoping and the verify trust anchor.

---

## Phase E — Supply chain, CI, and documentation truth

### STUDIO-19 — Wire the supply-chain gate; reconcile docs  · ~1–2 days · **F-7 + doc corrections**

**Goal.** The documented merge-blocking supply-chain gate actually runs in CI and passes
deterministically, and the two over-claims the audit caught are corrected at the source.

**Approach (F-7).**
- Add a CI job (new `.github/workflows/supply-chain.yml` or a step in existing workflows) that
  runs `cargo audit --ignore RUSTSEC-2025-0055` **and** `cargo deny check` on PRs and releases,
  failing the build on a regression.
- Reconcile `deny.toml` with the actual graph so a real `cargo deny check` passes: ignore the 6
  transitive unmaintained advisories (bincode, derivative, instant, paste, rand_os) with
  rationale mirroring `audit.toml`, and `allow-git` the pinned SSH source rev present in the
  lock — or document why each is denied. The intent: `cargo deny` is green and the green is real.

**Approach (doc corrections).**
- `COMPLETION_STATUS.md`: change "unwraps triaged (7 non-test, all provably safe)" → **9**
  (5 default-build + 4 core-live), all provably safe, per the audit §6.
- Ensure the "loudly-logged dev opt-in" language in `COMPLETION_STATUS.md:27`,
  `docs/journals/2026-06-04T0350…md`, and `core_bridge.rs:438` matches the STUDIO-18 code
  (loud + non-sticky). If STUDIO-18 chose the per-call flag, update the wording to describe that.
- Add an `.agentile/audits/` remediation-evidence pointer (Hybrid topology, per
  `AUDIT_REF.md`) linking each closed finding to its fix commit + test.

**Files.** `.github/workflows/*`, `deny.toml`, `audit.toml` (if rationale moves),
`COMPLETION_STATUS.md`, `docs/journals/2026-06-04T0350…md`, `src/core_bridge.rs` (comment),
`.agentile/audits/<id>/`.

**Acceptance.**
- [ ] CI runs `cargo audit` + `cargo deny` on PR + release and is green on this SHA.
- [ ] `cargo deny check` passes locally against the current `Cargo.lock` (no aspirational denies).
- [ ] The "9 unwraps" and "loud + non-sticky opt-in" statements are accurate everywhere they appear.
- [ ] A remediation-evidence record links F-1…F-7 → fix commit + proving test.
- [ ] Regression contract green.

**Regression risk + guard.** Lowest-risk sprint (CI + docs, no runtime behaviour change).
Guard: ensure the new CI job does not break the existing `release.yml`/`visual.yml` triggers;
the `cargo deny` reconciliation must not *hide* a real advisory — diff the ignore list against
`audit.toml` and justify each entry.

---

## Total

| Phase | Sprint | Findings | Est. (eng-days) |
|---|---|---|---|
| A — trust boundary | STUDIO-15 | F-1 | ~2–3 |
| B — auth hardening | STUDIO-16 | F-2 | ~1–2 |
| C — key storage | STUDIO-17 | F-3, F-5 | ~2–3 |
| D — capsule honesty | STUDIO-18 | F-4, F-6 | ~1–2 |
| E — supply chain + docs | STUDIO-19 | F-7 + corrections | ~1–2 |
| **Total** | **5 sprints** | **7 findings + 2 doc fixes** | **~7–12 eng-days** |

## Sequencing rationale

- **STUDIO-15 first** — highest severity + highest regression risk; it must absorb the most
  review and prove behaviour-preservation via the parity/e2e guards before anything else moves.
- **16 → 17 → 18** are independent of each other (auth / keys / capsules touch disjoint code),
  so they can parallelize if staffed, but are sequenced by descending shipping-build impact.
- **STUDIO-19 last** — it depends on 15–18 landing so the doc/CI truth reflects the final code.

## Risks + mitigations (remediation-specific)

- **F-1 rewrite regresses the legitimate approval flow.** *Mitigation:* relocate enforcement,
  don't redesign it; add rejection tests beside the existing accept tests; e2e loop is the gate.
- **F-3 roster serialization change breaks persistence/e2e.** *Mitigation:* keep `seed_demo` +
  injected-roster path for tests; only the secret field moves behind dev cfg.
- **Visual baseline drift from new banners (F-3, F-4).** *Mitigation:* treat each banner as a
  single deliberate baseline update with before/after in the close note; the other surfaces stay
  pinned — silent drift fails the sprint.
- **`cargo deny` reconciliation masks a real advisory.** *Mitigation:* every new ignore mirrors
  an `audit.toml` rationale; the diff is reviewed; `cargo audit` remains the independent check.
- **Decisions not yet made** (default first-run posture; localhost dev-issuer policy) block
  STUDIO-16/17. *Mitigation:* resolve `02_OPEN_QUESTIONS.md` Q1–Q3 before those sprints start.
