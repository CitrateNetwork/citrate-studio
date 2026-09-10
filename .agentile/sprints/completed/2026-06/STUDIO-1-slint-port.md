---
created: 2026-06-03T17:00:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: archived
sprint: STUDIO-1
---

# STUDIO-1 — Slint port of Citrate Studio

**Goal.** Port the `Citrate Studio.html` design prototype to a native Rust + Slint
application, 1:1 with the design system, with selectable/copyable text everywhere —
the foundation UI kit for every Citrate native app.

**Why now.** The design team produced a complete, high-fidelity prototype (React +
the Citrate Marketplace design system). Larry asked for a native Slint realization
that is pixel-faithful, responsive, and reactive — explicitly calling out that other
Citrate Slint apps don't let you highlight text, and that the kit should later
extend to `gui-native`. The prototype is the contract; this sprint makes it real on
the shared trust core.

**Scope.**
- A design-token foundation (`theme.slint`) 1:1 with `tokens.css` + `studio.css`.
- `SelectableText` (read-only `TextInput`) so content text is highlightable/copyable.
- The full path-based icon set + capability glyphs + risk/data-class/role primitives.
- The L0–L4 depth model: Beginner chat, the two-zone Composition Canvas over the
  sealed Audit Scrubber, the Approval Dock (four risk-tier presentations + hash pin +
  SoD), the Inspector, the Code Drawer.
- The ambient Health Strip (11 doctor checks, 9 tripwires), Break-Glass console,
  Settings + RBAC, and conversational onboarding.
- The playback state machine + all intents wired in Rust, honoring the trust
  boundary (UI submits; core decides).
- A headless software-render snapshot tool for fidelity review.

**Out of scope.** (→ deferred to the roadmap planset, STUDIO-2+)
- Wiring to the real `citrate-agent-core` (`cdylib`); the core is modeled in-process.
- Auth (OIDC/SIWE) and signer-key enrollment.
- Persistence, the real model resolver, live chain/audit data, packaging/signing.

**Repos affected.** `citrate-studio` (new).

**Plan.**

| # | Step | Status |
|---|---|---|
| 1 | Scaffold project (Slint 1.16, fonts, build) | done |
| 2 | Design tokens + typography + SelectableText | done |
| 3 | Icon set + UI-kit primitives | done |
| 4 | App shell: chrome, depth breadcrumb, health strip | done |
| 5 | Hero two-zone body: palette, canvas, scrubber, dock, inspector | done |
| 6 | Rust: data model, playback state machine, intents, SoD | done |
| 7 | Depth overlays: code drawer, health report, break-glass | done |
| 8 | Beginner chat, Settings + RBAC, conversational onboarding | done |

**Daily updates.**

- 2026-06-03 — kickoff. Fetched + studied the prototype bundle (every `.jsx`,
  `tokens.css`, `studio.css`, `studio-data.js`). Researched current Slint (1.16.1):
  confirmed `read-only TextInput` gives selection+copy (the highlight fix), and that
  responsiveness must be driven off `changed width` (no media queries). Scaffolded
  the project; validated the toolchain with a minimal two-zone window + headless
  snapshot. Built the token/typography/icon/primitive foundation.
- 2026-06-04 — built the shell, the hero two-zone body, and wired the Rust playback
  state machine + intents. Hit and resolved a cluster of Slint-specific issues
  (reserved property names `color`/`row`/`col`, `@children` under a conditional,
  binding loops on width-derived layout, and — the big one — a fully blank window
  caused by Slint elements defaulting to 100%-of-parent and a conditional `if` not
  being a stretchable layout slot). Stood up a headless software-renderer screenshot
  path to verify fidelity panel-by-panel. Built the three depth overlays, then the
  Beginner chat (with inline HITL approval), Settings + RBAC, and onboarding.
  Closed with a clean build (zero warnings) and verified every surface against the
  prototype.

**Decisions made.**

- ADR-2026-06-04-slint-renderer — software renderer (matches `gui-native`, headless
  snapshots, no GPU/skia build dep); femtovg/skia available as a one-flag swap.
- Slint **1.16.1** (latest), not the federation's 1.9 — this kit is the
  forward-looking base; backward-compatible via `compat-1-2`.
- Fonts embedded at compile time via `import "*.ttf"` (Slint's recommended path).
- The trust boundary is enforced in Rust: quorum/SoD/hash-pin computed in
  `main.rs`, handed to the UI as render-ready state. Swapping the modeled core for
  the real `agent-core` needs no UI change.

**Exit criteria.**

- [x] every plan step marked done
- [x] `cargo build` green, zero warnings
- [x] every prototype surface verified against the design (headless snapshots)
- [x] selectable/copyable text on all content surfaces
- [x] README + repo hygiene (LICENSE, AUDIT_TIER, audit.toml, deny.toml)
- [x] sprint closed and moved to `completed/`

**Close note.**

Landed the complete prototype as a native Slint app on the first pass, with high
fidelity confirmed by headless software-render snapshots of every panel and state
(idle / running / paused-at-gate / done, plus all overlays). The defining
requirement — highlightable text — is solved kit-wide via `SelectableText`. The
single largest cost was *not* the UI breadth but Slint's layout semantics: a blank
window burned real time before the root cause (default-100% sizing + a conditional
`if` not taking layout stretch) was isolated; the headless screenshot tool, once
built, made the rest of the work a tight see-verify loop. The honest gap: this is a
faithful *shell* over a *modeled* core — STUDIO-2+ (the roadmap planset) wires the
real `agent-core`, auth, and the new-user setup path. Full retro:
`docs/retrospectives/2026-06-04_STUDIO-1.md`.
