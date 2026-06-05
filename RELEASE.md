# Releasing Citrate Studio

The build is installer-ready (`cargo bundle`); the steps that require credentials
(code-signing, notarization) or people (external audit) are **gated**, not faked — they
run in CI only when their secrets are present, and are documented here so the path to a
signed `v1.0.0` is explicit.

## Local: a signed bundle on your machine

```sh
cargo install cargo-bundle
cargo bundle --release --format osx          # → target/release/bundle/osx/Citrate Studio.app
scripts/sign-macos.sh                        # Developer-ID sign + verify (+ notarize if creds)
```

`cargo bundle` produces the `.app` (ad-hoc signed). `scripts/sign-macos.sh` re-signs it with
your **Developer ID Application** cert (hardened runtime, timestamped,
`packaging/entitlements.plist`) and verifies it.

> **Keychain authorization.** `codesign` needs your keychain's private key. The first run
> prompts to allow `codesign` to use the key (click *Always Allow*) — so the signing step is
> **interactive** the first time and cannot run in a locked/headless session. In CI the cert is
> imported into an unlocked ephemeral keychain (see `release.yml`), so it's non-interactive
> there.

The app icon is `assets/icon/AppIcon.icns` (built with `iconutil` from the brand SVG); bundle
metadata is in `Cargo.toml` `[package.metadata.bundle]`.

### Notarization (the step after signing)

Needs Apple notary credentials. Store them once as a keychain profile, then sign+notarize:

```sh
xcrun notarytool store-credentials citrate \
  --apple-id <your-apple-id> --team-id DDHUG44QC7 --password <app-specific-password>
NOTARY_PROFILE=citrate scripts/sign-macos.sh   # signs, submits, staples
```

## CI: `.github/workflows/release.yml`

On a `v*` tag (or manual dispatch) the workflow, per desktop target
(macOS `aarch64`, Linux `x86_64`, Windows `x86_64`):

1. `cargo test --locked` (default features),
2. `cargo build --release --locked`,
3. `cargo bundle --release`,
4. **sign + notarize** — *only if the cert secrets exist* (see gates below),
5. upload the bundle as an artifact.

## The gates (what's required, and why it's not done here)

| Gate | Needs | How it's wired |
|---|---|---|
| **macOS sign + notarize** | An Apple Developer ID cert (`APPLE_CERT_P12`) + notary creds | CI step guarded by `env.APPLE_CERT_P12 != ''` — skipped without it; the unsigned `.app` still builds. |
| **Windows code-sign** | An Authenticode cert (`WINDOWS_CERT_PFX`) | CI step guarded on the secret. |
| **External audit attestation** | The federation auditor (AUDIT_TIER Tier 1) | Out-of-band against the release SHA — not a CI job; see `AUDIT_TIER.md`. |

These are credential/people gates, not code gates: the moment the secrets are configured
in the repo, the same workflow signs and notarizes with no code change.

## Before a real `v1.0.0`

Code-complete vs ship-complete are tracked in **`COMPLETION_STATUS.md`**. The functional
items still open before v1.0 (the canvas-driven live run + chain anchoring) are listed
there with what gates each — read it before tagging.
