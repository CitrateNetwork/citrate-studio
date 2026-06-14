# Citrate Studio — Partner Install Guide

**Release:** `v1.0.0-rc.1`
**Audience:** evaluation partners installing Citrate Studio on a desktop.

This guide gets you from a release download to a running, signed-in Studio. For
what's real vs preview, read **`docs/PARTNER_EVALUATION.md`** first.

---

## 1. What you receive

Per-platform desktop bundles produced by the release pipeline:

- macOS (aarch64): `Citrate Studio.app` (ad-hoc / unsigned in this RC)
- Linux (x86_64): `.deb` / AppImage
- Windows (x86_64): `.msi` (unsigned in this RC)

Plus checksums for each artifact.

> **Unsigned in this RC.** Code-signing and notarization are wired into CI and
> activate the moment signing certs are configured — no code change. Until then,
> your OS may warn on first launch (see Troubleshooting).

## 2. Requirements

- A supported desktop OS (macOS aarch64, Linux x86_64, or Windows x86_64).
- A **Citrate identity** to sign in (OIDC + SIWE via `auth.citrate.ai`). If you
  don't have one, request access from your Citrate contact.
- A local model runtime is optional — Studio probes for Ollama / llama.cpp and
  falls back to an embedded model if none is present.
- For on-chain anchoring: access to Citrate testnet (`https://rpc.citrate.ai`,
  chain 40204). Anchoring uses a real tx; a funded testnet account is needed only
  if you want to exercise the anchor step.

## 3. Install

**macOS**
```bash
# Verify checksum, then:
unzip "Citrate Studio.app.zip" -d /Applications/
# First launch: see Troubleshooting (unsigned app)
```

**Linux**
```bash
sudo dpkg -i citrate-studio_*.deb      # or run the AppImage directly
```

**Windows**
```
Run the .msi. Windows SmartScreen may warn (unsigned RC) — see Troubleshooting.
```

## 4. First run

1. **Sign in.** Studio opens a loopback OIDC + SIWE flow (RFC 8252 PKCE) against
   `auth.citrate.ai`. Approve in your browser; the token is stored in your OS
   keyring (never written to disk in plaintext).
2. **Onboarding** writes `citrate-studio.toml` (org id, model runtime, OIDC
   endpoints, policy & anchoring). The concierge walks you through it.
3. **Enroll your signer roster** (ed25519 keys) — these drive the approval quorum.

## 5. Run an evaluation

- **Stand up the harness** and dispatch the `hello` Capsule; confirm the output
  card shows a real wasmtime provenance badge.
- **Exercise the approval queue:** submit an action and route it through the
  quorum; observe separation-of-duties enforcement.
- **Replay the audit log** in the Audit Scrubber; confirm tamper detection.
- **(Optional) Anchor** the audit root to testnet (chain 40204) and confirm only
  the commitment appears on-chain.

## 6. Building from source (optional)

```bash
cargo test --locked                       # default features
cargo build --release --locked
cargo bundle --release --format osx       # macOS .app  (deb / msi on other OSes)
# To exercise the real agent core:
cargo build --release --locked --features core-live
```
See `RELEASE.md` for the full signing/notarization steps (credential-gated).

## 7. Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| macOS "cannot verify developer" | Unsigned RC app | Right-click → Open, or `xattr -dr com.apple.quarantine "/Applications/Citrate Studio.app"` |
| Windows SmartScreen warning | Unsigned RC `.msi` | "More info" → "Run anyway" |
| Sign-in never completes | Loopback port blocked / no browser | Allow the loopback redirect; retry in a default browser |
| No model runtime found | No Ollama/llama.cpp | Studio falls back to the embedded model — this is expected |
| Anchor step fails | No testnet access / unfunded account | Anchoring is optional for evaluation — skip it |
| Unverified-capsule banner | The dev opt-in is engaged | Expected in preview; the banner is intentional and non-sticky |

---

Questions or problems? See **`docs/PARTNER_FEEDBACK.md`**.
