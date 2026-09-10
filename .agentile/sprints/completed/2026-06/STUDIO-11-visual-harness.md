---
created: 2026-06-05T07:45:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: archived
sprint: STUDIO-11
---

# STUDIO-11 — Strong visual e2e harness

**Goal.** A harness that captures **every** studio surface/action as an image, builds a
browsable HTML gallery, and asserts nothing regressed visually (golden-image diff). "Confirm
everything works in images."

**The Playwright question — answered honestly.** Playwright drives web browsers
(Chromium/WebKit/Firefox); Citrate Studio is a **native Rust + Slint** app rendered by the
software renderer — there is no DOM for Playwright to attach to, so Playwright *cannot* drive
it. The native equivalent (and what this sprint builds) is a **snapshot harness** over the
headless software renderer: drive every state via env seeds, capture a PNG each, gallery +
diff. Playwright *is* the right tool for the **web** surfaces (the original `Citrate
Studio.html` prototype; a future cdylib-backed web re-skin) — a scaffold for that is included
and documented, pointed at where it applies.

**Approach.**
- `scripts/visual-harness.sh` — enumerate ~20 state specs (views, run states, overlays,
  settings sections, auth, scrubber tamper, onboarding); capture each via
  `CITRATE_STUDIO_SHOT` + the existing env seeds.
- `scripts/gallery.py` — generate `docs/visual/gallery/index.html` (labelled grid).
- `scripts/visual-diff.py` — golden-image regression vs `docs/visual/baseline/` (PIL mean
  pixel-diff with a threshold); `update` mode re-baselines.
- A `--features core-live` mode so the real-dispatch / real-doctor / real-chain states are
  captured against the real core.
- `tests/web-playwright/` scaffold + README for the web surfaces (honest scope).

**Plan.**

| # | Step | Status |
|---|---|---|
| 1 | `visual-harness.sh` — capture every surface | done |
| 2 | `gallery.py` — HTML gallery report | done |
| 3 | `visual-diff.py` — golden-image regression + baselines | done |
| 4 | core-live mode (real states) + Playwright web scaffold | done |
| 5 | Run it; commit baselines; document | done |

**Daily updates.**

- 2026-06-05 — kickoff. Native app ⇒ snapshot harness (not Playwright); Playwright scoped to
  web surfaces.
- 2026-06-05 — **shipped.** `visual-harness.sh` captures **19 surfaces** via the app's env
  seeds (run states, gate dock, completed run with real wasmtime badges under core-live,
  signed-in+KYC, inspector, code drawer, health, break-glass, chat, settings ×6, scrubber
  tamper, onboarding, beginner). `gallery.py` → a browsable HTML grid; `visual-diff.py` →
  PIL mean-pixel golden-image gate (**validated**: catches a swapped baseline at 5.74 > 2.0,
  writes a `*_diff.png` heat image). 19 baselines committed; gallery gitignored. `visual.yml`
  CI gate (macOS-pinned). Playwright scaffold in `tests/web-playwright/` for web surfaces,
  honestly scoped.

**Decisions made.**

- **Snapshot harness, not Playwright.** Playwright drives browsers; a native Slint app has no
  DOM, so it can't. Built the equivalent over the software renderer; scaffolded Playwright for
  the web surfaces where it applies. (See the essay.)
- **Golden images are platform-specific** (software-renderer AA) — CI pinned to macOS, with
  per-platform re-baselining documented.
- **Two harnesses, two questions:** STUDIO-8 asserts the *behavioral* loop; STUDIO-11 asserts
  every surface *renders right + hasn't drifted*.

**Exit criteria.**

- [x] The harness captures every studio surface/action as a PNG (19).
- [x] A browsable HTML gallery is generated.
- [x] Golden-image regression flags visual drift; 19 baselines committed; gate validated.
- [x] The Playwright-vs-native reality is documented; web scaffold present.

**Close note.**

STUDIO-11 answers "confirm everything works in images" with the right tool for a native app:
a software-renderer snapshot harness capturing all 19 studio surfaces, a browsable gallery,
and a golden-image regression gate proven to catch drift. The hard question — "run
Playwright" — is answered honestly in code and docs: Playwright drives browsers and cannot
attach to a native Slint app, so the equivalent-strength native harness is what ships, with a
documented Playwright scaffold reserved for the web prototype / a future web re-skin where it
genuinely belongs. Retro + journal + essay alongside.
