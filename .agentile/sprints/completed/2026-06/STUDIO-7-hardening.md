---
created: 2026-06-04T05:00:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: archived
sprint: STUDIO-7
---

# STUDIO-7 — Hardening pass: button up every edge

**Goal.** Leave no line unturned. Every `#[allow(dead_code)]` removed or precisely
justified; every stale comment corrected; every `unwrap`/`expect` in the live path
triaged for panic-safety; every honest seam (chain writes, PIV/FIDO2, upstream
CIT-AGENT-3/3e) either wired or cleanly documented. When we call this finished, it's
buttoned up to the max.

**Audit findings (the work-list).**
- **Dead scaffold**: STUDIO-2's `auth::{signer_id_from_pubkey, RosterEntry, enroll}` were
  superseded by STUDIO-4's `signing.rs`; `Claims.sub` + `AuthSession.kyc` unread;
  `FileTokenStore` is test-only now.
- **Unused-in-bin APIs**: `signing::Roster::authorized`, `chain::{CHAIN_ID,
  anchor_available}` — wire them into the bin (used, not dead).
- **5 module-level `#[allow(dead_code)]`** masking the above — remove once clean.
- **Stale comments**: the STUDIO-2 "pure for now / allow(dead_code) until wired" header.
- **54 `unwrap`/`expect`** in non-test src — triage: keep the provably-safe (mutex locks,
  the headless dev tool, infallible conversions), fix any live-path panic risk.

**Plan.**

| # | Step | Status |
|---|---|---|
| 1 | auth.rs: delete superseded scaffold; fix fields; gate test-only store | done |
| 2 | Wire `authorized` / `CHAIN_ID` / `anchor_available` into the bin | done |
| 3 | Remove module `#[allow(dead_code)]`; default build zero dead-code | done |
| 4 | unwrap/expect triage — fix live-path panic risks | done |
| 5 | Surface anchoring status; document chain-write + upstream seams | done |
| 6 | Tests for any changed logic; default + core-live green | done |

**Daily updates.**

- 2026-06-04 — kickoff + full audit. No TODO/FIXME; eprintlns are test-only. Work-list above.
- 2026-06-04 — **hardened.** Deleted the STUDIO-2 auth scaffold superseded by `signing.rs`
  (`signer_id_from_pubkey`, `RosterEntry`, `enroll`, `Claims.sub`); gated `FileTokenStore`
  to `#[cfg(test)]`; fixed stale comments. **Removed all 5 module `#[allow(dead_code)]`** —
  default build is dead-code-clean. Wired the unused-in-bin APIs: `authorized` (fail-closed
  check), `CHAIN_ID` (validate the live chain *is* 40204, flag a mismatch),
  `anchor_available` (Settings anchoring readiness). Wired the STUDIO-6 test-only bridges
  into the UI: **KYC** claim → a "✓ KYC" chip badge; the **real `DoctorReport`** → the
  Health Report + a computed strip status/summary + LED color; **real capsule dispatch** →
  a Settings "Run smoke capsule" button. Made the Account signer-id consistent with the
  real roster. **unwrap triage**: only 7 non-test sites — all provably safe (guarded env,
  post-`push` `last()`, single-thread mutex, entropy / tokio-runtime construction); no
  user-triggerable live-path panic. 34 default / 38 core-live tests; both builds zero studio
  warnings.

**Decisions made.**

- **`cfg_attr` over blanket `allow`** for genuinely cfg-conditional code:
  `#[cfg_attr(feature = "core-live", allow(dead_code))]` on the default-build signing path
  (`sign_for`/`keyring_secret`/`data::doctor` — the `core-live` binary signs via `LiveQueue`
  and shows the real doctor), and `#[cfg_attr(not(test), allow(dead_code))]` on a
  test-exercised accessor. These are accurate annotations, not suppression.
- **Wire, don't allow.** The STUDIO-6 bridges (dispatch, doctor) were proven by tests but
  unused in the bin — the no-shortcut fix is to surface them in the UI, not allow-dead them.
- **Keep real APIs.** `signature_count` mirrors the runtime's `signatures_on`; kept (with a
  precise non-test allow) rather than deleted to satisfy a lint.

**Exit criteria.**

- [x] Zero `#[allow(dead_code)]` except precisely-justified, narrowly-scoped `cfg_attr`.
- [x] No stale comments; every seam labelled with its real status.
- [x] Every live-path `unwrap`/`expect` triaged — all provably safe (7 non-test sites).
- [x] Default + core-live build green, zero studio warnings; 34 / 38 tests.

**Close note.**

STUDIO-7 buttons up the codebase. Every `#[allow(dead_code)]` is gone — replaced by deleting
genuinely-dead scaffold (the STUDIO-2 auth roster code `signing.rs` superseded), wiring the
test-only STUDIO-6 bridges (dispatch, doctor, KYC) into the live UI, and precisely annotating
the handful of cfg-conditional functions with `cfg_attr` instead of blanket suppression. The
unwrap audit found the "54" was test noise — only 7 non-test sites, all provably safe with no
user-triggerable panic. The live chain row now validates it really is 40204; the Health
Report shows the real doctor with a computed (not hardcoded) summary; the Account signer-id
matches the real roster. Both builds are studio-warning-free. The remaining honest seams are
upstream/infra and clearly labelled in-code: chain *writes* (gated on `CITRATE_ANCHOR_KEY`),
signed-capsule dispatch (CIT-AGENT-3e packer), and the L0 agent loop (CIT-AGENT-3). Retro +
journal + essay alongside.
