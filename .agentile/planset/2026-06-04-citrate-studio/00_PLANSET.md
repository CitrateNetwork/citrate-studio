---
created: 2026-06-04T21:45:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
sprint: citrate-studio roadmap
status: draft
---

# Planset — Citrate Studio (prototype → complete)

> **Purpose.** STUDIO-1 landed a faithful native Slint **shell** over a **modeled**
> core. This planset sequences the work to a **complete state**: every function backed
> by the real `citrate-agent-core`, the operator authenticated, and a **new user able
> to set up the agent and run it from scratch**. It is the contract for STUDIO-2+.

## Definition of "complete"

A new operator, on a clean machine, can:

1. **Sign in** with their Citrate identity (OIDC + SIWE via `citrate-identity`).
2. **Stand up the harness** through onboarding that *actually configures things*:
   discover a model runtime, enroll an approval roster (real keys), install signed
   capsules, set the oversight default — and have it **persist**.
3. **Run the agent** — a real `CapsuleDispatch` over real capsules, with real HITL
   approvals routed through the real `ApprovalQueue`, every step written to the real
   hash-chained `AuditChain` and anchored on chain 40204.
4. **Operate it day to day** in Studio — the Composition Canvas drives live runs, the
   Audit Scrubber replays real records, the Health Strip shows the real Doctor +
   tripwires, and Break-Glass is a real SecurityOfficer path.

When all four hold against the real core (not the in-process model), Studio is complete
for `v1.0.0` and enters the audit gate (`AUDIT_TIER.md`).

## What STUDIO-1 already delivered

The full prototype as a native app: the L0–L4 depth model, the two-zone composition,
the approval lattice (four risk tiers + hash-pin + SoD), the inspector, code drawer,
health report, break-glass, settings/RBAC, onboarding, the playback state machine — all
on a reusable UI kit, all selectable-text, zero warnings. See
`.agentile/sprints/completed/2026-06/STUDIO-1-slint-port.md`.

The boundary that makes the rest a *swap*: the UI is a viewport; quorum/SoD/hash-pin are
computed in Rust and handed over as render-ready state. Replacing the modeled core with
the real `cdylib` should need no `.slint` change.

## Reading order

| # | File | Covers | Read for |
|---|---|---|---|
| 00 | [`00_PLANSET.md`](00_PLANSET.md) | Index + "complete" definition (this file) | Orientation |
| 01 | [`01_SPRINT_SEQUENCE.md`](01_SPRINT_SEQUENCE.md) | STUDIO-2…8 with goals, estimates, acceptance | Engineering |
| 02 | [`02_OPEN_QUESTIONS.md`](02_OPEN_QUESTIONS.md) | Deferred decisions + assumptions | Decisions |

Auth has its own ADR + sprint (kicked off in parallel with this planset):
`.agentile/adrs/ADR-2026-06-04-auth-oidc-siwe.md` and
`.agentile/sprints/active/STUDIO-2-auth.md`.

## Executive summary — the gap

| Surface | STUDIO-1 (now) | Complete |
|---|---|---|
| Core | modeled in-process (`RunState`) | real `citrate-agent-core` (`cdylib`) |
| Auth | none | OIDC + SIWE via `citrate-identity`, OS-keyring session |
| Roster | demo signers | real key enrollment (file → keyring → PIV/FIDO2) |
| Run | scripted playhead | real `CapsuleDispatch` over signed `.cps` |
| Approvals | modeled `ApprovalQueue` | real `ApprovalQueue::add_signature` + attestation |
| Audit | demo frames | real `AuditChain` + on-chain anchoring (40204) |
| Doctor/tripwires | demo data | real `Doctor` report + tripwire daemon |
| Model | none (L0 stub) | real model resolver when CIT-AGENT-3 lands |
| Setup | onboarding theatre | onboarding that persists real config |
| Packaging | `cargo run` | signed native installers, web shell re-skin |

## What this planset does NOT cover

- **The agent loop / model resolver itself.** Those are `citrate-agent-core`'s
  CIT-AGENT-3, tracked in the runtime repo. Studio consumes them when they land; L0
  chat stays a designed surface with the local fallback until then.
- **The web shell.** The shared `cdylib` makes the web app a re-skin (per the design
  spec §6); it is a post-`v1.0.0` track, noted in 01 but not sequenced here.
- **New runtime features.** Studio surfaces what the runtime enforces; it does not add
  policy primitives.

## Versioning

- `v0.1.0` — STUDIO-1 (this commit): the faithful shell. Prerelease.
- `v0.2.0` — STUDIO-2 (auth) + STUDIO-3 (core wiring): real approvals + audit.
- `v0.3.0` — STUDIO-4/5 (roster enrollment + persistent new-user setup): a new user
  can set up from scratch.
- `v0.4.0` — STUDIO-6/7 (live data + interactivity/e2e): the agent runs for real.
- `v1.0.0` — STUDIO-8 (packaging) + the Tier-1 audit attestation.
