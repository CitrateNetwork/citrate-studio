---
created: 2026-06-04T06:40:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: archived
sprint: STUDIO-9
---

# STUDIO-9 — Packaging, signing, release

**Goal.** Make the build installer-ready and the release path reproducible, and map the
project honestly against the planset's "definition of complete" — so the gap to `v1.0.0`
is explicit and every remaining item is labelled with *what gates it*.

**Achievable here.**
- App-icon set rasterized from the brand SVG (`rsvg-convert`).
- `cargo-bundle` metadata (identifier, category, icon, license) — `cargo bundle` produces
  a `.app` / `.deb` / `.msi` skeleton.
- A release CI workflow (build + bundle the 3 desktop targets; sign/notarize steps gated
  on secrets, so they no-op without certs rather than fail).
- `RELEASE.md` (the build→bundle→sign→notarize→attest pipeline, with the gates marked).
- `COMPLETION_STATUS.md` mapping the definition-of-complete to real vs gated.

**Honest gates (cannot be done in this environment).**
- **Signing + notarization**: Apple Developer cert, Windows code-signing cert — secrets.
- **External audit attestation**: an external auditor (AUDIT_TIER Tier 1).
- **UI-kit cross-crate extraction consumed by another shell**: needs `gui-native` to exist;
  cross-crate Slint is a real refactor — structured, not forced.
- **Canvas-driven live run + chain anchoring**: the modeled playback → real
  `CapsuleDispatch` + `RecorderClient` wiring (a funded key) — the last live-operation
  piece.

**Plan.**

| # | Step | Status |
|---|---|---|
| 1 | Rasterize brand → app icon PNGs | done |
| 2 | `cargo-bundle` metadata + release profile (`strip`) | done |
| 3 | `release.yml` CI (build + bundle 3 targets; sign gated on secrets) | done |
| 4 | `RELEASE.md` + `COMPLETION_STATUS.md` | done |
| 5 | Default + core-live still green | done |

**Daily updates.**

- 2026-06-04 — kickoff. `rsvg-convert` present → real icon set achievable.
- 2026-06-04 — **shipped the achievable.** App icon set (6 PNGs, brand SVG → `rsvg-convert`);
  `[package.metadata.bundle]` (`cargo bundle` → `.app`/`.deb`/`.msi`) + release `strip`;
  `release.yml` (build + test + bundle × 3 targets, signing/notarization gated on secrets so
  they skip cleanly without certs); `RELEASE.md` (pipeline + gates) and `COMPLETION_STATUS.md`
  (the four operator capabilities mapped real/modeled/remaining/gated). README status pointer
  + module map. 35 / 39 tests, both builds zero warnings.

**Decisions made.**

- **Gate, don't omit.** Signing/notarization steps are written *fully* and guarded by
  `env.<SECRET> != ''` — dormant without certs (unsigned bundle still builds), self-activating
  with them.
- **Don't round "almost complete" up to "complete."** The strict definition needs the canvas
  live-run + anchoring; those are documented as remaining, so this is a *release candidate*.
- **Defer the UI-kit cross-crate extraction** rather than risk a green build — it's a real
  refactor whose acceptance needs `gui-native` to exist; structured + documented for later.

**Exit criteria.**

- [x] `cargo bundle` config present; icon set generated; build still green.
- [x] `release.yml` builds + bundles 3 targets; signing gated, not failing.
- [x] `RELEASE.md` + `COMPLETION_STATUS.md` map the path to v1.0 with every gate named.
- [x] Default + core-live green, zero warnings (35 / 39 tests).

**Close note.**

STUDIO-9 takes the project to a documented, installer-ready release candidate. The build
bundles to `.app`/`.deb`/`.msi`; the release workflow is reproducible with code-signing and
notarization wired as dormant, secret-gated steps that activate themselves when certs exist.
The headline ship items I structurally can't produce — signed installers (certs), the
external Tier-1 attestation (an auditor), the UI-kit consumed by another shell (`gui-native`)
— are wired/structured for and documented with what gates each. The functional gap to a true
`v1.0.0` (the canvas-driven live run + chain anchoring) is named precisely in
`COMPLETION_STATUS.md`. The honest deliverable of a packaging sprint, when the finish line
depends on the outside world, is a status map you can audit on paper — and that's the one
shipped here. Retro + journal + essay alongside; this closes the planset's Phase D and the
nine-sprint arc at a release candidate.
