# Citrate Studio

**The native agent-harness interface for the Citrate runtime — a Slint port of the
`Citrate Studio.html` design prototype, built as the forward-looking UI kit for every
Citrate native app.**

Citrate Studio is a creative tool for driving a compliance-first agent: it hides the
transformer at the top (L0 chat) and reveals the calldata at the bottom (L4 code), where
every layer down trades one abstraction for one truth. The compliance the runtime enforces
— hash-pinned approvals, the quorum lattice, the 11 doctor checks, the 9 tripwires, the
frame-accurate audit replay — becomes the most beautiful thing on screen rather than the
most buried.

It descends from the **Citrate Marketplace design system** (warm paper, citric-green
wax-seal accents, the lattice motif, mono compliance labels) and renders the
`citrate-agent-runtime` primitives directly: every value on screen traces to a struct,
enum, or function named in `CITRATE_STUDIO_DESIGN_SPEC.md`.

---

## What's here

This is a native Rust + **Slint 1.16** application. It is a **viewport + intent submitter**
(per the design spec's trust boundary): the UI renders state and submits intents; a Rust
"core" (modeled here with in-memory data + a playback loop) decides quorum / approval /
separation-of-duties. The UI never computes "approved."

### Built and verified

| Layer | Surface | Status |
|---|---|---|
| **Foundation** | design tokens (`theme.slint`), typography with **selectable text** (`SelectableText` = read-only `TextInput`, fixing the "can't highlight" gap in other Citrate Slint apps), the full path-based icon set, capability glyphs, risk/data-class/role/severity primitives, buttons, cards | ✅ |
| **Shell** | evergreen chrome, brand, L0→L4 depth breadcrumb, Beginner/Advanced workspace toggle, the always-ambient Health Strip (doctor LED + 9 tripwire lamps + break-glass), responsive column collapse | ✅ |
| **L1 Composition Canvas** | the timeline: lanes, clips with risk bands + capability glyphs, keyframe diamonds (Quorum gates), output cards blooming on completion, the sweeping playhead, transport, run status, dry-run / solo lane controls, the live tripwire-fired marker | ✅ |
| **Sealed Audit Scrubber** | the evergreen ledger zone: frame spine, `verify_integrity()` verdict, **simulate-tamper** → broken-link at the exact sequence, per-frame AuditRecord + RoleSignature detail | ✅ |
| **L3 Approval Dock** | the four risk-tier presentations (Low auto-feed · Medium ambient card · High quorum card with live signatures + SoD + the hash pin · Critical modal), the oversight dial with TTL countdown | ✅ |
| **L2 Inspector** | the AE twirl-down idiom over a capsule's Manifest (capabilities, data classes, risk, overlay, procedure, provenance → links to L4) | ✅ |
| **L4 Code Drawer** | the literal `manifest.toml` / `capsule.wit` / decoded calldata, with the live WIT/manifest revalidation (`wasm-linker-recheck`) blocker | ✅ |
| **Health Report** | the signed Doctor report (11 named checks) + the 9 FedRAMP tripwires with on-chain firing refs | ✅ |
| **Break-Glass Console** | the SecurityOfficer emergency path: phase spine, 72h affirmation countdown, the {Reviewer, ComplianceOfficer} quorum, the ITAR hard-block note | ✅ |
| **L0 Beginner chat** | the chat-first workspace — agent bubbles, quick replies, the scripted "run the reconciliation" flow with an **inline HITL approval card** (reusing the global gate + SoD), the outcome card, and "Open Studio" | ✅ |
| **Agent chat drawer** | the Advanced-mode L0 slide-in over the two-zone body | ✅ |
| **Settings + RBAC** | org→team→member roster with live add/drop (a dropped member keeps their personal workspace), signer roster, capability-grant AGT-14 drift, runtime/models, policy & anchoring | ✅ |
| **Conversational onboarding** | the first-run scripted setup — agent on the left, a live 6-step checklist + progress on the right; by the last step you've already prompted successfully | ✅ |
| **Playback state machine** | the `SPEED`/`TICK` loop (gate pause, medium queue, auto-feed, tripwire fire), all intents wired (sign / resume / approve / dry / solo / select / depth / chat / rbac / onboard) | ✅ |

The full prototype (`Citrate Studio.html`) is ported. Live-model chat (`window.claude.complete`)
is replaced by the prototype's in-character local fallback; wire it to the real agent loop
when `citrate-agent-core` lands its L0.

---

## Run it

```sh
cargo run
```

A native window opens with the **Advanced** two-zone workspace and the
`SOP-RECON-NIGHTLY` demo run. Press **play** (▶) in the canvas transport to watch the
playhead sweep: Low/Medium steps clear, the **High** PHI cross-match pauses for a 2-of-N
quorum (sign in the right-hand dock), `TRIP-AU-002` fires mid-run, and every step lands in
the sealed ledger below. Click a clip for the **Inspector**; the provenance twirl opens the
**Code Drawer**. Click the depth breadcrumb (L0…L4), the health strip, or BREAK-GLASS to
descend.

### Architecture

```
src/main.rs        — window setup, the RunState "core", playback loop, all intents, SoD,
                     headless-snapshot dev tool
src/data.rs        — the demo catalog (clips, 17 tools, capsules, frames, 11 doctor checks,
                     9 tripwires, signer roster) — mirrors studio-data.js
ui/theme.slint     — design tokens (1:1 with tokens.css + studio.css)
ui/typography.slint— SelectableText + text styles
ui/icons.slint     — AppIcon / CapGlyph (single-Path, 24×24 viewbox)
ui/primitives.slint— risk badges, data chips, role glyphs, keyframes, buttons, cards
ui/models.slint    — AppState global: data, run state, UI state, intents
ui/studio.slint    — the app shell (two-zone body, responsive, view + overlay routing)
ui/components/      — chrome, palette, canvas, scrubber, approval, inspector, overlays
                     (code/health/break-glass), chat (Beginner + drawer), settings, onboarding
assets/fonts/      — Geist, Geist Mono, Space Grotesk, Cormorant (embedded at compile time)
assets/brand/      — the C-mark
```

The trust boundary is explicit: **the front end never makes a policy decision.** Quorum,
SoD (CO ⊥ SO, Auditor never approves, no self-approval), and the hash-pin are computed in
`main.rs` and handed to the UI as ready-to-render state (`approval-roster`, `quorum-met`).
When this is wired to the real `citrate-agent-core` (`cdylib`), those computations move
behind the Rust core unchanged — the UI is already only a viewport.

### Dev: headless screenshots

This shell has no window surface, so the app renders headlessly via the software renderer:

```sh
CITRATE_STUDIO_SHOT=out.png \
  [CITRATE_STUDIO_SEED=gate|done|running] \
  [CITRATE_STUDIO_SELECT=c3] \
  [CITRATE_STUDIO_OVERLAY=code|health|breakglass|settings|chat] \
  [CITRATE_STUDIO_VIEW=onboard] [CITRATE_STUDIO_WS=beginner] \
  cargo run
```

Toggle **Beginner/Advanced** in the header; in Beginner, "Setup" replays the conversational
onboarding, and the settings gear opens **Settings + RBAC**.

---

## Renderer

Uses Slint's **software renderer** (winit backend) — matching `citrate-gui-native`,
pure-CPU, no GPU/`skia` build dependency, and it renders headlessly for review. Swap
`renderer-femtovg` (GPU) or `renderer-skia` (max text fidelity) in `Cargo.toml` for the
shipped desktop build if desired.

## Design fidelity

Built to match `Citrate Studio.html` 1:1: the warm-paper / evergreen two-zone law, the
risk-tier color+keyframe system, Space Grotesk titles / Geist UI / Geist Mono data /
Cormorant empty-state prose, hairlines over shadows, and **selectable, copyable text
everywhere it's content** (hashes, DIDs, outputs, code). Responsiveness is driven off the
window width via `changed` handlers (Slint has no media queries), collapsing the palette
and dock at narrow widths.
