---
created: 2026-06-04T00:00:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: active
---

# Audit Tier — `citrate-studio`

**Classification**: **Tier 1 — full audit** before first stable (`v1.0.0`) release tag.

## Rationale

Citrate Studio is the operator-facing native shell that drives the
`citrate-agent-runtime`: it renders the approval lattice, routes HITL signatures,
opens the break-glass path, and (forthcoming) authenticates operators and enrolls
their signing keys into the roster. It is a **viewport + intent submitter** — every
policy decision stays behind the Rust core — but it is the surface through which a
human authorizes consequential, on-chain, compliance-bearing actions. A flaw in how
it presents an approval, pins a payload hash, or stores an auth token has direct
trust consequences. It also embeds an auth client (OIDC/SIWE) and will hold session
tokens and, at enrollment time, touch local signing material.

## What this means concretely

- **Full code audit** by an external security firm before `v1.0.0`, covering: the
  intent/viewport boundary (no policy decision in the front end), the auth client
  (PKCE, token storage, JWKS validation, loopback redirect handling), local key
  enrollment into the `SignerRoster`, the dependency tree (`cargo audit` + `cargo
  deny`), the build supply chain, and CI/CD integrity.
- **No stable release tag** without a written audit attestation referencing the exact
  commit SHA.
- **Prerelease tags** (`v0.x.y-rc.N`) may ship without audit but MUST carry a
  `prerelease: true` flag on the GitHub Release and a clear warning in the notes.
- **Re-audit cadence**: every major version (`v1.0.0`, `v2.0.0`, …) AND any change
  that materially expands attack surface (the auth client, token storage, key
  enrollment, a new signing-surface integration, or wiring to a live `agent-core`).

## Decision authority

Per **D6** of the May 2026 federation-split decisions, every repo audits before its
first stable release. This document classifies what "audit" means for this repo.

Tier changes require: (a) a commit to this file explaining the change, AND (b)
sign-off from the operator listed in CODEOWNERS (when present) or from the federation
lead.

## See also

- `.agentile/AUDIT_REF.md` — this repo's pointer into the centralized federation
  audit trail (added as a new Tier-1 surface in the next audit cycle).
- Sibling tier docs: `citrate-gui-native/AUDIT_TIER.md`, `citrate-agent-runtime/AUDIT_TIER.md`.
