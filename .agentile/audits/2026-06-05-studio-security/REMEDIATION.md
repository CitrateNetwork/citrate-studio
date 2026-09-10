---
created: 2026-06-05T20:55:00Z
audit: Citrate Studio independent Tier-1 security review
audit_report: ../../../../handoffs/STUDIO_SECURITY_AUDIT_REPORT.md
audit_sha: 6f4d7c898ad70c3d89e100a4eb6e2a25f0218e62
planset: ../../planset/2026-06-05-studio-audit-remediation/
status: complete — default build + core-live verified; supply-chain CI secret wired (2026-06-06)
---

# Remediation evidence — Studio security audit (F-1 … F-7)

Local, next-to-the-code evidence (Hybrid topology, per `.agentile/AUDIT_REF.md`) mapping each
audit finding to its fix commit, the proving test, and residual verification.

| Finding | Sev | Sprint | Commit | Proving test(s) | Status |
|---|---|---|---|---|---|
| **F-1** SoD/dedup/self-approval in UI, not the policy seam | Med | STUDIO-15 | `6065e5a` | `default_build_seam_rejects_sod_double_sign_and_self_approval`, `apply_resume_is_a_noop_until_quorum_met` | ✅ fixed |
| **F-2** §3.1.3.7 JWKS-skip not enforcing the TLS precondition | Med | STUDIO-16 | `f55b5ae` | `issuer_scheme_enforced`, `empty_issuer_is_rejected`, `aud_array_membership_and_azp` | ✅ fixed |
| **F-3** plaintext keys on disk; auto-seeded FileBacked roster | Med | STUDIO-17 | `8e68fb9` | `production_to_json_never_writes_a_secret` | ✅ fixed |
| **F-5** `CITRATE_STUDIO_SIGNEDIN` fakes session in shipping | Low | STUDIO-17 | `8e68fb9` | (gated behind `debug_assertions`) | ✅ fixed |
| **F-4** silent, sticky `CITRATE_ALLOW_UNVERIFIED_CAPSULES` | Med (core-live) | STUDIO-18 | `12f7e4f` | (core-live) `real_hello_capsule_dispatches_through_wasmtime` (+ non-sticky env assertion) | ✅ fixed (core-live verified 2026-06-06) |
| **F-6** `verify_capsule` trusts the carried publisher key | Low/Info | STUDIO-18 | `12f7e4f` | `capsule_verify_rejects_unpinned_publisher` | ✅ seam added |
| **F-7** supply-chain gate not wired into CI; `deny.toml` drift | Med | STUDIO-19 | (this) | `.github/workflows/supply-chain.yml`; `cargo deny check` | ✅ wired |
| docs | Info | STUDIO-19 | (this) | "7 unwraps" → 9; "loudly-logged" now true (STUDIO-18) | ✅ corrected |

## Regression posture (per the planset contract, at each sprint close)

- Default build: `cargo build` + `cargo clippy --all-targets` clean (zero warnings).
- Tests: 35 (audit SHA) → **42** (none removed/weakened; +7 proving tests).
- Parity + e2e invariant suites green under the default build throughout.
- Visual harness: 19/19; one deliberate baseline refresh (`12-settings-roster`, F-3 banner).
- `cargo audit --ignore RUSTSEC-2025-0055`: 0 vulnerabilities throughout.

## Residual verification — completed 2026-06-06 (DevOps, network-connected + chain SSH)

Both SSH-gated gates from the original handoff
(`handoffs/STUDIO_DEVOPS_VERIFICATION_HANDOFF.md`) are now closed. Verified against `main`
@ `1643b05` with the real chain source resolved over SSH
(`git+ssh://git@github-citrate-chain/CitrateNetwork/citrate-chain?rev=0f2d16b…`) and the local
`../citrate-agent-runtime/agent/core` path dep (its lock pins the same chain rev).

**Task A — `core-live` build + tests.**
- `cargo build --features core-live`: ✅ finished (3m11s). Studio's own crate compiles with
  **zero warnings**. The only 2 warnings are upstream in the `citrate-agent-core` path dep (an
  undeclared `insecure-dev-hitl` cfg gate) — not Studio code, not a remediation regression.
- `cargo test --features core-live`: ✅ **46 passed / 0 failed / 0 ignored**.
- **F-1 parity** — `dock_routes_through_the_real_live_queue` ✅: Studio's enrolled keys are
  accepted by the real async `ApprovalQueue` (verify + RM-G.1 roster + SoD + quorum), confirming
  the default-build `admit_signature` mirror matches the authoritative core.
- **F-4 non-sticky env** — `real_hello_capsule_dispatches_through_wasmtime` ✅, run with
  `--nocapture`: the gate opens (`WARN: opening the capsule integrity gate
  (CITRATE_ALLOW_UNVERIFIED_CAPSULES=1) …`), real wasmtime dispatch returns `Hello, Aleia` (not
  skipped), and the new post-dispatch assertion
  `assert!(std::env::var("CITRATE_ALLOW_UNVERIFIED_CAPSULES").is_err())` passes — the bypass is
  restored to unset, never sticky. The assertion now enforces the F-4 property in CI rather than
  relying on inspection.

**Task B — supply-chain CI secret + workflow fix.**
- Local pre-validation of the exact CI commands (chain source resolved over SSH, sibling tree
  present): `cargo audit --ignore RUSTSEC-2025-0055` → exit 0 (only non-failing unmaintained
  warnings, incl. the already-ignored `rand_os` via `bip39`); `cargo deny check` → exit 0
  (advisories/bans/licenses/sources all ok). The `*-not-detected` / `license-not-encountered`
  notes are the documented superset and are non-failing.
- **First real CI run revealed a workflow bug** (the gate had never executed before — it was
  authored but unrun). `cargo-audit` passed (it reads the committed `Cargo.lock`, which already
  covers the entire core-live graph incl. chain crates — full advisory coverage, no secret even
  needed). `cargo-deny` **failed**: it runs `cargo metadata`, which must read the **path-dep**
  manifest `../citrate-agent-runtime/agent/core/Cargo.toml` — and that sibling tree is absent on
  the runner. The original workflow assumed deny would pull the chain *git+ssh* source directly;
  the real blocker is the missing sibling path dep, upstream of any SSH fetch.
- **Fix (option A — full graph in CI):** `.github/workflows/supply-chain.yml` `cargo-deny` job
  now (1) provisions **two** read-only deploy keys — `CITRATE_CHAIN_DEPLOY_KEY` (chain crates) +
  `CITRATE_AGENT_RUNTIME_DEPLOY_KEY` (the sibling), each confirmed to match the key registered
  read-only on its repo; (2) clones `citrate-agent-runtime` to the sibling path at the pinned rev
  `95b4dd4` (whose own lock pins the same chain rev `0f2d16b…` as studio's); (3) runs
  `cargo deny check` against the complete core-live graph. Both secrets live on
  `CitrateNetwork/citrate-studio`. The only private git source in either lock is
  `github-citrate-chain`, so no other aliases are needed.
- A second workflow fix was needed once the sibling resolved: cargo's libgit2 backend ignores
  `~/.ssh/config` host aliases, so it could not resolve `github-citrate-chain` (DNS failure on the
  literal alias). Set `CARGO_NET_GIT_FETCH_WITH_CLI=true` so cargo shells out to the system `git`
  (which honours the alias + IdentityFile — the same git the sibling-clone step uses).
- **Green on a real PR (#6):** workflow run `27078661898` — `cargo audit` ✅ and `cargo deny check`
  ✅ (`advisories ok, bans ok, licenses ok, sources ok`) against the complete core-live graph,
  with the chain source resolved over SSH and the agent-runtime sibling checked out at `95b4dd4`.
- **Maintenance note:** `AGENT_RUNTIME_REV` in the workflow is a cross-repo pin. Bump it whenever
  studio's `Cargo.lock` advances its chain rev, keeping the chosen agent-runtime rev's own lock on
  the same chain commit.

All `audit/studio-15…19-*` branches were already merged to `main` (@ `1643b05`); only this
verification PR (F-4 assertion + this record) remains.
