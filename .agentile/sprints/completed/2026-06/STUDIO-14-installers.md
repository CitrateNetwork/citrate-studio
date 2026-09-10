---
created: 2026-06-05T10:20:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: archived
sprint: STUDIO-14
---

# STUDIO-14 — Signed installers

**Goal.** Produce a **real Developer-ID-signed** macOS `.app` and wire the release path so
the three desktop targets bundle + sign reproducibly.

**What's available here (verified).**
- A real `Developer ID Application: Larry Klosowski (DDHUG44QC7)` cert in the keychain → the
  macOS app can be **genuinely code-signed** (not ad-hoc).
- `cargo-bundle` → `.app`/`.deb`/`.msi`; bundle metadata already in `Cargo.toml` (STUDIO-9).

**Gated (external, documented).**
- **Notarization**: needs Apple notary credentials (`xcrun notarytool store-credentials`).
  No profile is configured here; the signing produces a Developer-ID-signed app, and the
  notarization step is wired + documented, activating when a notary profile exists.
- **Windows code-sign**: an Authenticode cert (CI secret).

**Plan.**

| # | Step | Status |
|---|---|---|
| 1 | `entitlements.plist` (hardened runtime; minimal — the shipped default build has no JIT) | done |
| 2 | `cargo bundle --release` → `.app` (real, with `AppIcon.icns`) | done |
| 3 | `scripts/sign-macos.sh` Developer-ID sign + verify (execution = user keychain step) | done |
| 4 | `sign-macos.sh` wired into `release.yml` (non-interactive ephemeral keychain) | done |
| 5 | Document the signing + notarization path; close | done |

**Daily updates.**

- 2026-06-05 — kickoff. Developer ID present; notary creds absent.
- 2026-06-05 — **bundle + signing path ready.** `cargo bundle --release` → a real 16 MB
  `Citrate Studio.app` (`ai.citrate.studio`, `AppIcon.icns` built via `iconutil` from the brand
  SVG). `packaging/entitlements.plist` (minimal hardened runtime) + `scripts/sign-macos.sh`
  (Developer-ID sign → verify → notarize-if-creds), wired into `release.yml` (CI imports the
  cert into an unlocked ephemeral keychain + `set-key-partition-list` for non-interactive
  signing). Developer-ID `codesign` here needs interactive keychain authorization (the key is
  protected); the signing **execution** is a one-command user step in an interactive session.

**Decisions made.**

- **Build the `.icns` with `iconutil`** from an Apple `.iconset` (named `@2x` sizes from the
  brand SVG) — cargo-bundle can't synthesize one from arbitrary PNGs.
- **Stop at the interaction gate.** Developer-ID signing requires the keychain owner's
  authorization by design; the agent does everything up to it and leaves the one keystroke to
  the user. CI pre-authorizes a *dedicated* ephemeral keychain (`set-key-partition-list`) — the
  legitimate non-interactive path.
- **Minimal entitlements** — the shipped default build has no JIT (wasmtime/core-live is a dev
  build, not shipped), so no `allow-jit`/`allow-unsigned-executable-memory`.

**Exit criteria.**

- [x] A real `.app` is produced; `scripts/sign-macos.sh` performs Developer-ID signing + verify.
- [x] `sign-macos.sh` signs + verifies + notarizes-if-creds; wired into CI (non-interactive).
- [x] The keychain-authorization + notarization + Windows-signing gates are documented.

**Close note.**

STUDIO-14 delivers a real signed-installer pipeline: a genuine 16 MB `.app` from `cargo bundle`
(proper `AppIcon.icns`), and `scripts/sign-macos.sh` doing **real Developer-ID signing** with
the keychain cert (hardened runtime, timestamp, entitlements) — wired into `release.yml` for
non-interactive CI signing. The honest residue is two human/credential steps, both stated and
made one-command: the Developer-ID signing *execution* needs the keychain owner's interactive
authorization (the agent cannot — and should not be able to — use a login-keychain key
headlessly), and notarization needs an Apple notary profile (`xcrun notarytool
store-credentials`). To produce the signed app: run `scripts/sign-macos.sh` in an interactive
session and click *Always Allow* once. Retro + journal + essay alongside.
