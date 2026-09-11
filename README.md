# citrate-studio

*Part of the **[Citrate Network](https://citrate.ai)** — own the means of computation. · [Docs](https://docs.citrate.ai) · [Run a node](https://citrate.ai/download) · [Contribute → free membership](https://github.com/CitrateNetwork/.github/blob/main/CONTRIBUTING.md)*

> The native agent-harness desktop app for the Citrate Network — drive a compliance-first agent, with the transformer at the top (L0 chat) and the calldata at the bottom (L4 code).

## What it is

Citrate Studio is a Rust + [Slint](https://slint.dev) native desktop application: the interface for driving a Citrate agent and the reference UI kit for every Citrate native app. It renders the runtime's compliance primitives — hash-pinned approvals, the quorum lattice, doctor checks, tripwires, and frame-accurate audit replay — directly on screen, while a Rust core computes every policy decision (the front end never makes one). By default Studio runs a *modeled* in-process core; under the `core-live` feature it delegates to the real [`citrate-agent-core`](https://github.com/CitrateNetwork/citrate-agent-runtime) runtime.

It anchors audit roots to the Citrate chain (id **40204**) and authenticates against [citrate-identity](https://github.com/CitrateNetwork/citrate-identity) via OIDC/SIWE. See the concept overview at https://docs.citrate.ai/apps.

## Prerequisites

Studio is a pure Cargo/Slint build — **no Node.js, no GPU, no OpenSSL** (Slint uses its software renderer; auth uses `ureq` + `rustls`).

```bash
# Rust stable (pinned by rust-toolchain.toml) + Cargo
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup component add rustfmt clippy

# Linux: an OS keyring service (GNOME Keyring / KWallet) is used to store OIDC tokens.
# macOS/Windows use the system keychain — no extra packages.

# For packaging installers (optional):
cargo install cargo-bundle --locked
```

No system GPU/skia/OpenSSL packages are required.

## Build from source

```bash
git clone https://github.com/CitrateNetwork/citrate-studio
cd citrate-studio

# Default build — modeled (in-process) core, no wasmtime, fast:
cargo build

# Production binary — stripped, LTO, opt-level 3:
cargo build --release --locked
# Artifact: target/release/citrate-studio

# Build against the REAL agent runtime (pulls wasmtime + citrate-wallet-core):
cargo build --features core-live

# Tests:
cargo test --locked                    # default (modeled) core
cargo test --features core-live        # parity against the real core
```

Packaged installers land under `target/release/bundle/**`:

```bash
cargo bundle --release --format deb    # Linux .deb  (or: osx | msi)
# macOS -> "Citrate Studio.app", Windows -> .msi; bundle id ai.citrate.studio
```

## Run locally

Studio is a native desktop app — it opens a window, it does **not** serve an HTTP port.

```bash
cargo run
```

This opens the two-zone workspace with a demo run loaded. To verify a headless build renders correctly, capture a screenshot instead of opening a window:

```bash
CITRATE_STUDIO_SHOT=out.png cargo run
# Optional demo seeding: CITRATE_STUDIO_SEED=gate|done|running
#                        CITRATE_STUDIO_OVERLAY=code|health|breakglass|settings|chat
```

A non-empty `out.png` confirms the UI kit built and rendered.

## Connect it locally

Studio is a desktop client that talks to three upstreams. By default it points at the public testnet (`rpc.citrate.ai`, `auth.citrate.ai`) and a modeled core, so it runs standalone out of the box. To wire it to a **local** stack:

1. **Chain RPC (chain 40204)** — Studio reads/anchors against a JSON-RPC endpoint. It ships pointed at `https://rpc.citrate.ai`; to exercise the live path locally, run a local devnet node from [citrate-chain](https://github.com/CitrateNetwork/citrate-chain) and opt into the live RPC/anchor path:

   ```bash
   export CITRATE_LIVE_RPC=1          # run the live RPC check
   export CITRATE_ANCHOR_LIVE=1       # perform a real on-chain anchor
   export CITRATE_ANCHOR_KEY=<hex-privkey-funded-on-40204>
   ```

2. **Identity / OIDC** — point Studio's OIDC issuer at a locally-running [citrate-identity](https://github.com/CitrateNetwork/citrate-identity):

   ```bash
   export CITRATE_STUDIO_ISSUER=http://localhost:3000   # default: https://auth.citrate.ai
   ```

   Studio uses loopback PKCE (RFC 8252) with `client_id=citrate-studio`; tokens are stored in the OS keyring.

3. **Agent core** — to drive the real runtime instead of the modeled one, build with `--features core-live` (path dep on `../citrate-agent-runtime/agent/core`) and point capsules at a local directory:

   ```bash
   cargo run --features core-live
   export CITRATE_CAPSULES_DIR=./capsules
   ```

4. **Local model runtime (optional)** — Studio auto-discovers local model servers for L0 chat: [Ollama](https://ollama.com) at `http://localhost:11434` and llama.cpp at `http://localhost:8080`. Start one to enable live model output.

For the full multi-repo bring-up see `LOCAL_STACK.md` in [citrate-docs](https://github.com/CitrateNetwork/citrate-docs).

## Configuration

Studio has **no `.env` file** — configuration is environment variables plus a TOML written to the platform config dir (`…/citrate-studio/citrate-studio.toml`) during onboarding.

| Variable | Default | Purpose |
|---|---|---|
| `CITRATE_STUDIO_ISSUER` | `https://auth.citrate.ai` | OIDC issuer (identity) |
| `CITRATE_LIVE_RPC` | unset | `=1` runs the live RPC test against the configured node |
| `CITRATE_ANCHOR_LIVE` | unset | `=1` performs a real on-chain anchor (chain 40204) |
| `CITRATE_ANCHOR_KEY` | — | signing key for on-chain anchoring |
| `CITRATE_CAPSULES_DIR` | — | capsule (skill) source directory |
| `CITRATE_STUDIO_SHOT` | unset | render one headless screenshot to the given path and exit |

Chain constants are compiled in: `RPC_URL=https://rpc.citrate.ai`, `CHAIN_ID=40204` (`src/chain.rs`). Dev/demo seeding vars (`CITRATE_STUDIO_SEED`, `_OVERLAY`, `_VIEW`, `_WS`, …) are documented in `src/config.rs`.

## Links

- Docs: https://docs.citrate.ai/apps
- Depends on: [citrate-agent-runtime](https://github.com/CitrateNetwork/citrate-agent-runtime) (agent core) · [citrate-identity](https://github.com/CitrateNetwork/citrate-identity) (OIDC/SIWE) · [citrate-chain](https://github.com/CitrateNetwork/citrate-chain) (RPC, chain 40204)
- Contributing (DCO): CONTRIBUTING.md · Security: SECURITY.md · License: LICENSE

## License

Source-available (BUSL-1.1) — free for personal/non-commercial use; commercial use requires a Citrate membership. This is **not** an open-source license.
