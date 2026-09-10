---
sprint: STUDIO-16
title: Enforce the TLS precondition + tighten claim validation (audit F-2)
created: 2026-06-05T19:40:00Z
branch: audit/studio-16-auth-tls
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: closed
planset: ../../../planset/2026-06-05-studio-audit-remediation/
audit_finding: F-2 (Medium)
---

# STUDIO-16 — Enforce the §3.1.3.7 TLS precondition + tighten claim validation

> Closes audit finding **F-2 (Medium)**: the OIDC §3.1.3.7 JWKS-skip is sound only over a
> TLS-authenticated channel, but `CITRATE_STUDIO_ISSUER` could set an `http://` issuer with no
> scheme check — silently removing the TLS that authenticates the never-signature-checked ID
> token. Plus two secondary gaps: `validate_claims` skipped the issuer check on an empty
> issuer, and `aud` was matched on the first array element only (no membership / `azp`).

## Spec (decisions locked — see planset 02, Q2)

- Release builds **require an `https://` issuer**; a plaintext loopback issuer
  (`http://127.0.0.1` / `localhost` / `[::1]`) is accepted **only under `#[cfg(debug_assertions)]`,
  and loudly** (`WARN`). Any other non-https issuer is rejected, fail-closed.
- `validate_claims` rejects an **empty configured issuer** (no silent skip).
- Audience is validated by **membership** (client_id ∈ `aud`), and a **multi-audience** token
  requires `azp == client_id` (OIDC Core §3.1.3.7).

## Changes

- **`src/auth.rs` — `AuthConfig::validate_issuer()`**: https-only, with the debug-only loud
  loopback exception. Wired into `login()` (before opening the browser) and `refresh()` (before
  trusting a refreshed token) — the two token-acquiring entry points.
- **`Claims`**: replaced the single `aud: String` with `audiences: Vec<String>` + `azp:
  Option<String>` (the unused primary-`aud` field was removed rather than `#[allow]`-ed).
- **`decode_claims`**: collects all audiences (string-or-array) + `azp`.
- **`validate_claims`**: rejects empty issuer; checks audience membership; enforces `azp` for
  multi-audience tokens.
- **`src/main.rs` `auth_config`** doc + **`COMPLETION_STATUS.md` §1**: state the enforced
  https precondition + the membership/`azp` rules.

## Tests (added)

- `issuer_scheme_enforced` — https ok; non-loopback http rejected even in debug; loopback http
  ok under `debug_assertions`.
- `empty_issuer_is_rejected` — an empty configured issuer fails (no silent skip).
- `aud_array_membership_and_azp` — client_id present-but-not-first is valid; absent is rejected;
  multi-aud without matching `azp` is rejected; single-aud needs no `azp`.
- Existing `decode_and_validate_claims` updated to the `audiences` model (deliberate tightening).

## Regression gate

- `cargo clippy --all-targets` — **zero warnings**.
- `cargo test` — **40 passed / 0 failed** (was 37; +3). All existing auth tests still pass,
  incl. `pkce_s256_rfc7636_vector`, `loopback_capture_and_token_exchange_against_a_mock` (the
  mock uses `exchange_code` directly — unaffected by the `login`/`refresh` issuer guard).
- `scripts/visual-harness.sh check` — **19/19 match baseline** (no UI change).
- `cargo audit --ignore RUSTSEC-2025-0055` — clean (no dep change).

## Residual / follow-ups

- `core-live` compile not run here (SSH-gated) — auth.rs is feature-independent, so no
  core-live-specific risk; confirm with `cargo test --features core-live` in a connected env.
- `logout()` still posts a bearer token to `{issuer}/logout` without the issuer guard; it
  sends (not trusts) a token, so it is lower-risk, but a future tidy could guard it too.
