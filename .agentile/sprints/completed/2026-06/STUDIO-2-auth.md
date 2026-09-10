---
created: 2026-06-04T22:05:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: archived
sprint: STUDIO-2
---

# STUDIO-2 — Authentication (OIDC + SIWE)

**Goal.** An operator signs in with their Citrate identity (`citrate-identity` /
`auth.citrate.ai`); Studio holds a validated session, surfaces "signed in as 0x…", and
has the seam to turn that identity into a rostered signer (STUDIO-4).

**Why now.** A new user can't set up the agent from scratch without first being *who
they say they are*. Auth is the front door to the from-scratch setup path, and it can be
built in parallel with the core wiring (STUDIO-3) because it touches a separate surface
(a Sign-In screen + a session, not the approval lattice).

**Scope.**
- The pure, compiling auth scaffold (`src/auth.rs`): PKCE (S256), `AuthConfig` +
  `authorize_url`, `TokenSet`, claims parse, `TokenStore` trait + file-backed dev impl,
  `signer_id_from_pubkey`, the wallet→roster `RosterEntry` mapping. Unit-tested.
- The network client (behind new deps, one reviewable commit): loopback listener
  (`tiny_http`), token exchange + JWKS validation + userinfo (`reqwest`,
  `jsonwebtoken`), keyring storage (`keyring`).
- A Slint **Sign-In** surface + "signed in as 0x…" in the chrome; logout.

**Out of scope.** Role enrollment + signing surfaces (STUDIO-4). Attested roles (post
v1.0.0). A dedicated `citrate-studio` client registration in `citrate-identity`
(coordinate; Q2).

**Repos affected.** `citrate-studio`; a one-line client registration in
`citrate-identity` (Q2).

**Plan.**

| # | Step | Status |
|---|---|---|
| 1 | ADR (`ADR-2026-06-04-auth-oidc-siwe.md`) | done |
| 2 | Pure scaffold `src/auth.rs` (PKCE, tokens, store, signer_id) + tests | done |
| 3 | Loopback listener + browser open + callback capture | done |
| 4 | Token exchange + claims validation (iss/aud/exp) | done |
| 5 | Keyring token storage + refresh-on-expiry + logout | done |
| 6 | Slint sign-in surface + chrome session indicator | done |
| 7 | `citrate-identity`: register `citrate-studio` client (Q2) | done |
| 8 | dep audit + `audit.toml`/`deny.toml` rationale for new deps | done |

**Refinement (decided during execution).** Dropped JWKS/`jsonwebtoken`: for the native
code flow the ID token is received directly from `/token` over a TLS channel we
initiated, so per **OIDC Core §3.1.3.7** it is trusted via TLS server authentication and
re-verifying the RS256 signature is unnecessary. We validate `iss`/`aud`/`exp` as defense
in depth. This removes a heavy crypto dependency from the Tier-1 surface. The SSE
logout-cascade (`/sessions/events`) is deferred — it's a multi-app nicety, not core to
sign-in; logout already revokes server-side + clears local tokens.

**Daily updates.**

- 2026-06-04 — kickoff in parallel with the roadmap planset. ADR accepted (loopback
  PKCE + SIWE, keyring storage, identity→role as separate enrollment). Landed the pure
  scaffold: PKCE S256 (verified against a known RFC 7636 test vector), `authorize_url`,
  token model, `FileTokenStore`, `signer_id_from_pubkey` (SHA-256, matches the
  runtime), and the `RosterEntry` mapping — all unit-tested, zero warnings. Network +
  keyring + UI are the next steps.

- 2026-06-04 — **finished the network/keyring/UI.** Landed the full native flow in
  `auth.rs`: ephemeral 127.0.0.1 loopback (`tiny_http`), system-browser launch (no dep),
  callback capture with state check, code→token exchange (`ureq`/rustls), TLS-trusted ID
  token + `iss`/`aud`/`exp` validation, `KeyringTokenStore` (OS keyring), `logout`
  (server revoke + clear), `refresh`-on-expiry, and `session_from_store` for startup. The
  Slint chrome gained a session chip (Sign in / Signing in… / wallet `0x…` / failed —
  retry); sign-in runs off-thread → `invoke_from_event_loop`. 9 auth tests (incl. a
  loopback capture + a token exchange against an in-process mock); 17 total, zero
  warnings. `cargo audit` triaged: 1 advisory (RUSTSEC-2025-0055, agent-core's tracked
  Low, only via the optional `core-live` tree — not in the default build) documented in
  `audit.toml` + `deny.toml`; 6 informational unmaintained warnings recorded. The
  `citrate-studio` OIDC client was registered in `citrate-identity` (native, loopback).

**Decisions made.**

- ADR-2026-06-04-auth-oidc-siwe — loopback PKCE (RFC 8252), not device flow; tokens in
  OS keyring; auth keys ≠ signer keys.
- Drop JWKS/`jsonwebtoken` — the code-flow ID token is TLS-trusted from the direct
  `/token` response (OIDC Core §3.1.3.7); validate iss/aud/exp instead. Smaller Tier-1
  trust surface.

**Exit criteria.**

- [x] Sign-in: browser → loopback callback → token exchange → ID-token validated →
      stored in the OS keyring (flow complete; logic mock-tested in-process).
- [x] Slint sign-in surface + the wallet chip in the chrome; working logout + refresh.
- [x] `audit.toml` + `deny.toml` updated with rationale; new deps triaged (no
      vulnerabilities in the default build).
- [x] `citrate-studio` client registered in `citrate-identity`.
- [x] Sprint moved to `completed/` with a close note.

**Close note.**

STUDIO-2 ships a complete native loopback-PKCE auth client against `citrate-identity`:
sign-in (browser → loopback callback → code exchange → TLS-trusted ID token → keyring),
logout, refresh-on-expiry, and a session chip in the chrome — all off the UI thread, all
parity with the explorer/dashboard web RPs. The notable refinement was dropping
JWKS/`jsonwebtoken`: a native code flow gets its ID token directly from `/token` over
TLS, so per OIDC Core §3.1.3.7 it's trusted via the channel; we validate iss/aud/exp
instead. The honest limit: the *live* end-to-end (a real running identity server) is
reached via `CITRATE_STUDIO_ISSUER` but isn't exercised in CI — the flow's logic is
covered by in-process loopback + `/token` mocks, not a live IdP. That live integration
test belongs to a federation-level e2e harness (identity + studio together), noted for
STUDIO-5/integration. The identity→role enrollment (turning the authenticated wallet into
a rostered signer with real keys) is STUDIO-4, by design (auth keys ≠ signer keys). Retro:
`docs/retrospectives/2026-06-04_STUDIO-2.md`.
