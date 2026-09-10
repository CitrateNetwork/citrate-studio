---
created: 2026-06-04T21:00:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: active
repo: citrate-studio
tier: T1
---

# Agent Entry — citrate-studio

> **Lightweight subset.** This file points back to the canonical Agentile
> framework lineage. Start here whenever you (human or AI) are working in
> **citrate-studio**.

## What this repo is

The native agent-harness interface (Rust + Slint) that drives the
`citrate-agent-runtime`. A 1:1 port of the `Citrate Studio.html` design
prototype, and the forward-looking **UI kit** for every Citrate native app
(gui-native, a Fortune-200 aerospace partner shell, learning-center). It is a **viewport + intent
submitter**: it renders runtime state and submits intents; every policy
decision (quorum, approval, separation-of-duties, capability, integrity)
stays behind the Rust core.

Repo tier: **T1** — full audit before `v1.0.0`. See `AUDIT_TIER.md`.

## What to read, in order

1. **This file** (you're here).
2. **README.md** — what's built, how to run, the architecture.
3. **Active roadmap** — [`.agentile/planset/2026-06-04-citrate-studio/00_PLANSET.md`](planset/2026-06-04-citrate-studio/00_PLANSET.md).
   The sequenced path from "prototype port" to "a new user can set up the
   agent and run it from scratch."
4. **Federation control plane** — `citrate-federation/agentile/AGENT_ENTRY.md`
   (the active control-plane entry) and `rules/CORE_RULES.md` (non-negotiables).
5. **Sibling runtime** — `citrate-agent-runtime/.agentile/AGENT_ENTRY.md`. The
   core this shell drives; its `CITRATE_STUDIO_DESIGN_SPEC.md` maps every UI
   element to a runtime primitive.

## Audit lineage

This repo participates in the federation-wide audit cadence. It is a **new
Tier-1 surface** (created 2026-06-04), to be folded into the next federation
audit cycle. See `.agentile/AUDIT_REF.md`.

## What lives here, locally

| Path | Purpose |
|---|---|
| `.agentile/AGENT_ENTRY.md` | This file — entry point. |
| `.agentile/AUDIT_REF.md` | Two-way link into the federation audit trail. |
| `.agentile/sprints/` | Repo-scoped sprints (`active/`, `backlog/`, `completed/YYYY-MM/`). |
| `.agentile/adrs/` | Repo-local architectural decision records. |
| `.agentile/planset/` | The architecture-first roadmap (numbered docs). |
| `docs/journals/` | Timestamped deep-dives on discoveries. |
| `docs/retrospectives/` | Sprint-end reviews (metrics + honest assessment). |
| `docs/essays/` | Long-form reflections on patterns and lessons. |
