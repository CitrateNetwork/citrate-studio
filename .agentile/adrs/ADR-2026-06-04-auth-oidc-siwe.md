---
created: 2026-06-04T22:00:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-2
---

# ADR-2026-06-04 — Authentication: OIDC loopback PKCE + SIWE via citrate-identity

## Status

Accepted (design); implementation in STUDIO-2.

## Context

Citrate Studio is a **native desktop app** that must authenticate an operator and turn
that identity into a role-bearing signer in the agent's `SignerRoster`. The federation's
auth authority is **`citrate-identity`** (`auth.citrate.ai`) — a panva `oidc-provider`
with **SIWE (EIP-4361)** login. Findings from the identity exploration:

- It is **OIDC standard**: `/.well-known/openid-configuration`, `/jwks` (RS256),
  `/auth` (Authorization Code + **PKCE S256 required**), `/token`, `/userinfo`,
  `/logout`, `/sessions/events` (SSE logout cascade), plus `/siwe/challenge` +
  `/siwe/verify`.
- The `sub` / `wallet_address` claim **is** the identity (EIP-55 wallet address).
  Roles are **not** issued by auth — they live in the app.
- It registers `citrate-explorer` + `citrate-dashboard` as trusted public clients with
  a **loopback redirect** (`http://127.0.0.1:<port>/auth/callback`, RFC 8252) already
  allowed — the native-app pattern.
- The **device flow (RFC 8628) is not implemented** (S4). It does **not** issue or hold
  the ed25519 keys the roster expects.
- `citrate-explorer` is the reference RP: generate PKCE, redirect to `/auth`, capture
  `code`, exchange at `/token`, store the ID token, read `wallet_address`.

## Decision

1. **Loopback PKCE (RFC 8252), not device flow.** Studio spins an ephemeral
   `127.0.0.1:<port>` listener, opens the system browser to `/auth` with
   `code_challenge=S256`, captures `code`+`state` on the loopback callback, and
   exchanges at `/token` with the `code_verifier`. This is the production-ready path the
   web RPs already use; device flow can be adopted later when identity ships S4.
2. **SIWE wallet sign-in** happens in the browser interaction (identity drives it);
   Studio never touches the wallet key — it only receives the resulting tokens.
3. **Validate the ID token** (RS256) against `/jwks` before trusting any claim; extract
   `wallet_address` + `kyc_status` from `/userinfo` (live KYC, not the frozen token).
4. **Store tokens in the OS keyring** (macOS Keychain / Windows Credential Manager /
   libsecret), never plaintext. Refresh via `grant_type=refresh_token`; logout via
   `POST /logout`; subscribe to `/sessions/events` to honor cascaded logout.
5. **Register a dedicated `citrate-studio` client_id** in `citrate-identity` (clean
   audit + revocation), with the loopback redirect. Reuse `citrate-explorer` only as a
   bring-up shortcut (Open Question Q2).
6. **Identity → role is a separate, explicit enrollment** (STUDIO-4). Auth gives a
   wallet address; a workspace admin maps that wallet's ed25519 **pubkey** → role(s) in
   `StaticSignerRoster`; `signer_id = SHA-256(pubkey)`. Auth keys ≠ signer keys: the
   wallet authenticates the human; the signer key (file → keyring → PIV/FIDO2) signs
   approvals and is stamped `SigningSurfaceTag::Slint`.

## Consequences

- The front end stays a viewport: it submits a signed attestation; the core's
  `ApprovalQueue` + `SignerRoster` decide. Auth adds *who* the operator is, not *what
  they may approve*.
- New deps arrive in a single reviewable commit: `reqwest` (token/JWKS/userinfo),
  `tiny_http` (loopback listener), `jsonwebtoken` (RS256 verify), `keyring` (secrets),
  `url`, `sha2`+`base64` (PKCE). `cargo deny` + `audit.toml` updated with rationale —
  the Tier-1 audit surface grows here, deliberately and visibly.
- The pure scaffold (`src/auth.rs`: PKCE, token model, `TokenStore`, claims parse,
  `signer_id`) lands first and is unit-tested; the network/keyring wiring follows behind
  the deps so the auditor reviews the trust-bearing code as one diff.
- A `citrate-studio` client registration is a one-line change in `citrate-identity`
  config — coordinate before v0.2.0.

## References

- `citrate-identity/src/{config,siwe,siwe-routes,logout-routes}.ts`
- `citrate-explorer/src/app/auth/callback/page.tsx` + `src/lib/auth/config.ts` (the RP)
- `citrate-agent-runtime/agent/core/src/hitl/signing.rs` (`SignerRoster`,
  `signer_id_from_pubkey`), `audit/record.rs` (`SigningSurfaceTag`)
- RFC 8252 (native app loopback), RFC 7636 (PKCE), EIP-4361 (SIWE)
