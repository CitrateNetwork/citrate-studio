---
sprint: STUDIO-17
title: Production key storage by default; gate dev seeds (audit F-3, F-5)
created: 2026-06-05T20:10:00Z
branch: audit/studio-17-key-storage
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: closed
planset: ../../../planset/2026-06-05-studio-audit-remediation/
audit_finding: F-3 (Medium), F-5 (Low)
---

# STUDIO-17 — Production key storage by default; gate dev seeds

> Closes **F-3 (Medium)** — the shipped default build seeded 5 FileBacked signers on first
> run and persisted their **plaintext ed25519 private keys** to `roster.json`, and those keys
> satisfied real gates — and **F-5 (Low)** — `CITRATE_STUDIO_SIGNEDIN`/`_KYC` fabricated an
> authenticated session in the shipping binary.

## Spec (decisions locked — planset 02: Q1, Q3 / Q2-adjacent Q for F-5)

- **First run seeds nothing** in a shipping build (Q1): High/Critical fail closed until the
  operator enrolls signers (onto Keyring) deliberately. The demo seed moves behind a new
  `dev-filebacked` feature.
- **No private key on disk in production** (Q3): secret serialization is dev/test-only;
  production `roster.json` carries pubkey/role/signer_id/surface/wallet only.
- **Insecure-storage banner** in Settings → Signer Roster whenever a FileBacked signer exists.
- **`CITRATE_STUDIO_SIGNEDIN`/`_KYC`** are honored only under `#[cfg(debug_assertions)]`.

## Changes

- **`Cargo.toml`** — new `dev-filebacked` feature (demo/screenshot seed; never shipped).
- **`src/signing.rs`**
  - `to_json()` is now the **production** serializer — never a secret. New
    `to_json_with_secrets()` (`cfg(any(test, feature = "dev-filebacked"))`) persists secrets
    for the demo seed + round-trip tests; both share `to_json_impl(include_secrets)`.
  - `load_or_seed()` — shipping returns `Roster::default()` (fail-closed) on first run;
    `dev-filebacked` seeds + persists the demo roster (with secrets).
  - `seed_demo()` gated to `cfg(any(test, feature = "dev-filebacked"))` (dev/test affordance).
- **`src/main.rs`** — `refresh()` sets `roster-insecure` (any FileBacked signer);
  `restore_session` + `headless_shot` read `CITRATE_STUDIO_SIGNEDIN` only under
  `debug_assertions`.
- **UI** — `models.slint` adds `roster-insecure: bool`; `settings.slint` RosterSection renders
  the danger banner when set. (Deliberate visual change — see baseline note.)

## Tests

- Added `production_to_json_never_writes_a_secret` — `to_json()` emits no private-key value and
  no secret survives a production round-trip, while pubkey/role/id persist.
- Updated `roster_persist_roundtrip` to use `to_json_with_secrets()` (deliberate: the
  production serializer intentionally drops secrets; the dev path keeps them).

## Regression gate

- `cargo clippy --all-targets` — **zero warnings**.
- `cargo test` — **41 passed / 0 failed** (was 40; +1 net).
- `cargo build --features dev-filebacked` — compiles clean.
- `scripts/visual-harness.sh check` — **19/19 match**. Exactly one **deliberate** baseline
  update: `12-settings-roster.png` (mean diff 2.61) for the new insecure-storage banner; the
  other 18 surfaces are byte-stable. Baseline refreshed for surface 12 only.
- `cargo audit` — unchanged.

## Notes / residual

- The dev machine's existing `roster.json` (written by the pre-fix format) still contains
  FileBacked secrets, so the banner renders there — which is why surface 12 changed. A clean
  shipping install now writes no secret and seeds no signer.
- A runtime "Dev key" (FileBacked) enrollment in a shipping build is not persisted across
  restart (its secret isn't written); the banner warns. Keyring enrollment is unaffected
  (secret lives in the OS keyring).
- `core-live` compile not run here (SSH-gated); signing.rs is feature-independent of core-live.
- Visual harness still builds default features; a clean-environment demo should build/run with
  `--features dev-filebacked` to populate the roster (demo fingerprints regenerate per fresh
  seed — a pre-existing property of `seed_demo`).
