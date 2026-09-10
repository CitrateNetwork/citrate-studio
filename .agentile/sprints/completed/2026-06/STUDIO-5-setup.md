---
created: 2026-06-04T01:30:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: archived
sprint: STUDIO-5
---

# STUDIO-5 — New-user setup that persists

**Goal.** Onboarding stops being theatre: each of the 6 steps configures something
**real**, the configuration is written to a `citrate-studio.toml` that **survives a
restart**, and first launch routes to onboarding while subsequent launches go straight
to Studio.

**Approach.**
- A `config` module: a persisted `citrate-studio.toml` (workspace/tenant, runtime,
  oversight default, capsules installed, first prompt, `onboarding_complete`).
- First-launch routing: `onboarding_complete` absent ⇒ onboarding; present ⇒ Studio.
- Wire the 6 steps to real actions: workspace/tenant → config; **model-runtime
  discovery** (real HTTP probe of Ollama :11434 + llama.cpp :8080, embedded Gemma
  fallback); roster enrollment (STUDIO-4, Keyring default); **capsule install** with
  real ed25519 signature verification, **fail-closed** on unverified; oversight default.
- Pay the STUDIO-4 debts: inject the roster into `new_state` (no `$HOME` I/O in unit
  tests); Keyring is the default enrollment surface.

**Out of scope.** Real `.cps` bytes from a live registry (STUDIO-6 dispatch); the live
agent loop for the first prompt (STUDIO-6); real model inference.

**Plan.**

| # | Step | Status |
|---|---|---|
| 1 | `config` module: `citrate-studio.toml` persist/load + tests | done |
| 2 | Runtime discovery (probe :11434/:8080) + capsule verify (fail-closed) | done |
| 3 | First-launch routing (config detected ⇒ Studio) | done |
| 4 | Wire the 6 onboarding steps to real actions + persist | done |
| 5 | Pay STUDIO-4 debts: inject roster into `new_state`; Keyring default | done |
| 6 | Tests + screenshots; default build green | done |

**Daily updates.**

- 2026-06-04 — kickoff. Building the `config` module first.
- 2026-06-04 — **shipped.** `src/config.rs`: a persisted `citrate-studio.toml` (flat TOML,
  no serde-derive), real runtime discovery (HTTP probe of Ollama :11434 + llama.cpp :8080,
  embedded fallback), and fail-closed capsule install (real ed25519 verify; the rogue
  capsule is rejected). The 6 onboarding steps drive real actions via a pure
  `onboard_apply` and persist; the runtime prompt reflects a real probe. First-launch
  routing: `!config::is_configured()` ⇒ onboarding, else Studio. STUDIO-4 debts paid:
  `new_state_with` injects roster + config (no `$HOME` in unit tests); Keyring is the
  default enrollment surface. 7 new tests (31 total); default + core-live builds green,
  zero warnings. Verified: onboarding renders; config round-trips; capsule fail-closed and
  runtime probe are tested against an in-process mock.

**Decisions made.**

- **Hand-rolled flat TOML** for `citrate-studio.toml` instead of `serde`-derive + the
  `toml` crate — the config is six flat keys; a dependency wasn't worth it. (Cost a real
  escaped-quote bug, caught by the round-trip test; see the journal.)
- **Discovery reports, doesn't assert** — the runtime step probes for real and says what
  answered (usually "nothing, using embedded"), rather than the old scripted "I found a
  few on your network."
- **Fail-closed at install time** — `install_capsules` verifies publisher signatures and
  stages only what checks out; the rogue capsule never enters the config.
- **The roster step confirms the STUDIO-4 roster**; the capsule sources are a demo set and
  the first prompt is captured but not yet run — those executions are STUDIO-6.

**Exit criteria.**

- [x] Clean machine → onboarding → a working, persisted `citrate-studio.toml`
      (`onboarding_builds_a_persistable_config` drives all 6 steps; config round-trips).
- [x] First launch shows onboarding; subsequent launches go to Studio (`is_configured()`
      routing in `main()`; `CITRATE_STUDIO_FRESH` forces onboarding).
- [x] Runtime discovery is a real HTTP probe (surfaced, not faked); embedded fallback.
- [x] Capsule install verifies signatures and fails closed (rogue rejected; tested).
- [x] `new_state_with` injects roster **and** config — no unit test reads `$HOME`.
- [x] Default + core-live builds green, zero warnings; tests cover config + probe + verify.

**Close note.**

STUDIO-5 makes onboarding *mean* something: each of the six steps performs a real action
and writes a `citrate-studio.toml` that survives restart, and the second launch routes
past setup into Studio — the one behavior a setup flow can't fake. Discovery is an honest
HTTP probe that reports what answered (usually "nothing, binding embedded Gemma") instead
of the old scripted line; capsule install is fail-closed at install time, verifying
ed25519 publisher signatures and rejecting the rogue capsule before it can enter the
config. The two STUDIO-4 debts are paid in the very next sprint — `new_state_with` injects
both the roster and the config (no `$HOME` I/O in unit tests), and Keyring is the default
enrollment surface. The honest edge, stated plainly: setup persists a real, verified
*intent*; the *execution* of that intent — binding the runtime to a live inference loop,
dispatching real `.cps`, running the first prompt through a real agent — is STUDIO-6.
Retro: `docs/retrospectives/2026-06-04_STUDIO-5.md`; journal + essay alongside.
