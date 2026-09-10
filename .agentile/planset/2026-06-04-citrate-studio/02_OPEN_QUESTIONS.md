---
created: 2026-06-04T21:55:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
sprint: citrate-studio roadmap
status: draft
---

# 02 — Open Questions

Deferred decisions and assumptions. Each gets an ADR when resolved.

## Q1 — Native shell: keep Slint, or wrap the core in Tauri?
STUDIO-1 is Slint (matches `gui-native`, hardware-attested signing surfaces the roster
needs). The design spec floats a Tauri shell wrapping the same `cdylib` for maximum
web-component reuse. **Assumption:** stay Slint; revisit only if the web re-skin
(post-v1.0.0) makes shared components compelling. Owner: Larry.

## Q2 — Auth client_id: reuse `citrate-explorer` or register `citrate-studio`?
`citrate-identity` registers `citrate-explorer` + `citrate-dashboard` as trusted
first-party public clients with a loopback redirect. STUDIO-2 can reuse
`citrate-explorer` initially, but a dedicated `citrate-studio` client_id (with its own
loopback redirect) is cleaner for audit and revocation. **Assumption:** register
`citrate-studio` in `citrate-identity/src/config.ts` before v0.2.0. Owner: identity +
studio.

## Q3 — Identity → role mapping: manual, or attested?
`citrate-identity` authenticates a *wallet*, not a *role*. For MVP, a workspace admin
maps an authenticated wallet's pubkey → role in the roster (STUDIO-4). The S3 identity
registry (DIDs, on-chain role SBT) could later attest roles. **Assumption:** manual
admin enrollment for v1.0.0; attested roles are post-v1.0.0. Owner: studio + identity.

## Q4 — Where do signing keys live, and which surfaces for v1.0.0?
The roster needs ed25519 pubkeys; `citrate-identity` does **not** issue/hold them.
Tiered surfaces: `FileBacked` (dev), OS keyring, PIV/CAC, FIDO2. **Question:** which are
mandatory for v1.0.0 vs. stubbed? **Assumption:** file + keyring mandatory; PIV/FIDO2
seam present but may slip to v1.1 if hardware-attestation plumbing in the runtime
(`SigningSurface` trait) isn't ready. Owner: studio + runtime.

## Q5 — Config persistence format + secret handling.
`citrate-studio.toml` for non-secret config (workspace, runtime endpoints, roster
pubkeys, policy); OS keyring for tokens + any private material. **Question:** per-user
vs. per-machine; multi-workspace switching. **Assumption:** per-user under the platform
config dir; multi-workspace is a v1.1 nicety. Owner: studio.

## Q6 — "Run in the dashboard": is Studio the dashboard, or does it embed in
`citrate-dashboard`?
The ask was "the agent runs in the dashboard for the user." **Interpretation:** Studio
*is* the operator dashboard for the agent (the Composition Canvas is the run surface).
The Next.js `citrate-dashboard` is network monitoring, a separate surface. **Assumption:**
Studio is the agent's operating dashboard; no embed into `citrate-dashboard`. Confirm
with Larry. Owner: Larry.

## Q7 — L0 chat: real model when, and which?
L0 chat depends on the runtime's agent loop + model resolver (CIT-AGENT-3, not landed).
**Assumption:** keep the in-character local fallback, clearly labelled "forthcoming,"
and wire the real loop in STUDIO-6 *only when* CIT-AGENT-3 ships (Ollama / llama.cpp /
embedded Gemma per the resolver). Don't fake a model. Owner: runtime gates this.

## Q8 — Audit-cycle inclusion.
citrate-studio is a new Tier-1 surface created after the 2026-05-31 deep audit. **Action:**
add it to the next federation audit cycle (`citrate-security`), populate the live
`audit_id` in `.agentile/AUDIT_REF.md`. Owner: security + studio.
