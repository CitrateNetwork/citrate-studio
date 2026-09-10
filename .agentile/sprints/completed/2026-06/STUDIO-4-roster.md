---
created: 2026-06-05T00:10:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: archived
sprint: STUDIO-4
---

# STUDIO-4 — Signer roster + real ed25519 key enrollment

**Goal.** Turn an authenticated identity into a role-bearing **signer with a real
ed25519 key**: enroll/disenroll signers per role, persist the roster, and make the
approval dock produce **real, verifiable signatures** over the gate payload — replacing
the demo roster's fake fingerprints.

**Why now.** STUDIO-2 established *who* the operator is (a wallet). STUDIO-3 made the
core decide policy. STUDIO-4 produces the *artifact* the policy acts on — a real
attestation from a real key — and is the prerequisite for the live async `ApprovalQueue`
(STUDIO-6), which verifies exactly these signatures.

**Approach.**
- A `signing` module: ed25519 keypair gen / sign / verify; `signer_id = SHA-256(pubkey)`
  (byte-identical to the runtime). `SigningSurface` tiers — **FileBacked** (dev) +
  **Keyring** (OS keyring private key) working; **PIV/CAC** + **FIDO2** stubbed behind a
  clear seam (the `SigningSurfaceTag::Slint` attested-hardware path the spec calls for).
- A persisted **roster** (enroll/disenroll, JSON in the platform config dir; private
  keys for keyring surfaces in the OS keyring). Seed the 5 demo signers with real
  generated FileBacked keypairs on first run so the dock works with real crypto.
- The approval dock candidates come from the enrolled roster; signing produces a real
  ed25519 signature over the gate's payload, **verified** before it counts.
- **Fail-closed**: a required role with no enrolled signer cannot satisfy the gate
  (surfaced in the dock + the roster section).
- Settings → **Signer Roster** becomes interactive (enroll/disenroll, surface choice).

**Out of scope.** The live async `ApprovalQueue` attestation verification (STUDIO-6);
PIV/FIDO2 hardware (stub); the consolidated `citrate-studio.toml` config (STUDIO-5).

**Repos affected.** `citrate-studio`.

**Plan.**

| # | Step | Status |
|---|---|---|
| 1 | `signing` engine: ed25519 gen/sign/verify + signer_id + tests | done |
| 2 | `SigningSurface` tiers (FileBacked, Keyring; PIV/FIDO2 stub) | done |
| 3 | Roster: enroll/disenroll + persistence + seed demo keys | done |
| 4 | Wire dock candidates + real-signature signing | done |
| 5 | Settings roster UI: enroll/disenroll, fail-closed surface | done |
| 6 | Tests + screenshot; default build green | done |

**Daily updates.**

- 2026-06-05 — kickoff. Building the signing engine first (real ed25519).
- 2026-06-04 — **shipped.** `src/signing.rs`: ed25519 gen/sign/verify, `signer_id =
  SHA-256(pubkey)` (runtime-identical), `SigningSurface` tiers (FileBacked + Keyring
  sign; PIV/FIDO2 → typed `SurfaceNotWired` seam), a persisted `Roster` (enroll/disenroll,
  JSON in the platform config dir; Keyring secrets in the OS keyring), seeded with the 5
  demo signers' real keys. Wired the dock: candidates come from the roster (fp = real
  signer_id), `on_sign` produces a real signature over the capsule payload hash, **verified
  before it counts**; fail-closed when a required role has no signer. Settings → Signer
  Roster is interactive (enroll Keyring/dev, disenroll, surface badge). 7 new tests (24
  total), default + core-live builds green, zero warnings. Verified by screenshot: the
  dock + Settings show identical real signer_ids (e.g. Reviewer `6d5e…4628`).

**Decisions made.**

- **Seed real keys for the demo** rather than start empty — first run generates the 5
  signers' keypairs (FileBacked) and persists them, so the approval flow runs on real
  crypto immediately. Keyring is one click away in Settings.
- **The signature is the artifact; the decision stays in the policy seam.** STUDIO-4
  produces + verifies a real attestation; the real async `ApprovalQueue` that *consumes*
  attestations against pinned payloads is STUDIO-6. The quorum decision still routes
  through STUDIO-3's `policy::quorum_satisfied`.
- **FileBacked = dev surface** (private key in a local JSON file, named as such); Keyring
  stores the secret in the OS keyring. STUDIO-5 will default new enrollments to Keyring.

**Exit criteria.**

- [x] Enroll/disenroll a signer; the roster persists; the dock candidates + SoD reflect it.
- [x] FileBacked + Keyring surfaces sign; PIV/FIDO2 stubbed with a clear seam.
- [x] A real ed25519 signature from an enrolled key verifies and counts toward a High gate
      (in-memory; the live async `ApprovalQueue` verification is STUDIO-6).
- [x] Fail-closed: a required role with no signer cannot satisfy the gate.
- [x] Default build green, zero warnings; tests cover gen/sign/verify + roster persist.

**Close note.**

STUDIO-4 makes the signer real. An enrolled role now carries a generated ed25519 keypair;
`signer_id = SHA-256(pubkey)` is the same value in the dock, the Settings roster, and the
runtime's `SignerRoster`. Pressing Sign produces an actual signature over the capsule's
payload hash, verified before it counts; disenroll a role and the gate fails closed in
front of you. Signing surfaces are tiered and honest — FileBacked + Keyring sign for real,
PIV/CAC + FIDO2 are a typed `SurfaceNotWired` seam, not a fake. The roster persists to the
platform config dir (Keyring secrets to the OS keyring). Two debts are recorded for
STUDIO-5: FileBacked-by-default (Keyring should be the default once persistent config
lands) and the config-dir I/O inside `new_state` (inject the roster for clean unit tests).
The deferred piece by design: the real async `ApprovalQueue` *consuming* these attestations
is STUDIO-6 — STUDIO-4 produces the artifact, STUDIO-3 owns the decision, STUDIO-6 will own
the verifier. Retro: `docs/retrospectives/2026-06-04_STUDIO-4.md`.
