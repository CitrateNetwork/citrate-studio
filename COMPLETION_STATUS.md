# Citrate Studio — completion status

Maps the planset's **definition of "complete"**
(`.agentile/planset/2026-06-04-citrate-studio/00_PLANSET.md`) to what is real today, what is
modeled, and what gates each remaining item. Honest by design: a green row means *real
against the runtime under `core-live`*, not "looks done."

Legend: ✅ real · 🟡 modeled (default) / real (core-live) · ⏳ remaining · 🔒 gated on infra/people

## The four operator capabilities

### 1. Sign in — ✅
OIDC + SIWE against `citrate-identity` via the native loopback-PKCE flow (RFC 8252).
Real token exchange, TLS-trusted ID token (OIDC §3.1.3.7 — the JWKS-skip is enforced to be
sound: release builds require an `https://` issuer, a plaintext loopback issuer is debug-only
and loudly logged; `aud` is validated by membership with `azp` for multi-audience tokens),
keyring storage, refresh, logout, a session chip + KYC badge. *(STUDIO-2, STUDIO-16)*

### 2. Stand up the harness — ✅
Onboarding configures real, persisted state in `citrate-studio.toml`: a real model-runtime
probe (Ollama/llama.cpp + embedded fallback), real ed25519 roster enrollment, fail-closed
signed-capsule install, oversight default. First launch → onboarding; thereafter → Studio.
*(STUDIO-4, STUDIO-5)*

### 3. Run the agent
| Piece | Status | Note |
|---|---|---|
| HITL approvals via the real `ApprovalQueue` | ✅ | studio's ed25519 keys → real attestations → verify + RM-G.1 roster + SoD + quorum; the live dock routes through it under `core-live`. *(STUDIO-4, 6)* |
| Real `CapsuleDispatch` (wasmtime) | ✅ | the `hello` capsule executes; Settings "Run smoke capsule". Real `.cps` *signing* awaits the upstream packer (CIT-AGENT-3e); today's run uses a logged dev opt-in. *(STUDIO-6)* |
| Real hash-chained `AuditChain` | ✅ | the scrubber's `verify_integrity` + the Doctor run against a real chain. *(STUDIO-3, 6)* |
| **Canvas drives the live run** (real dispatch on clip completion, real `ToolResult` badge) | ✅ | *(STUDIO-10)* each completed clip runs a real capsule via wasmtime; the output card shows a "⚡ wasmtime" provenance badge with the real return. The run dispatches the real `hello` smoke (the `recon.*` fleet awaits the upstream packer); heavy-capsule off-thread dispatch is noted. |
| **Anchoring on chain 40204** | ✅ | *(STUDIO-12)* the audit root is anchored in a **real tx on chain 40204** via the runtime's `RecorderClient` (EIP-155). Proven on-chain: root `0x50d89f48…` → tx `0xb89dde7e…`, block 497881, root verified in calldata. Settings → Policy has an "Anchor audit root" button. Non-rostered keys anchor via a self-tx (the `AgentDecisionRegistry` write methods are recorder-gated); the funded key came from `faucet.citrate.ai`. |

### 4. Operate day to day
| Surface | Status |
|---|---|
| Audit Scrubber replays real records / detects tamper | ✅ *(core-live)* |
| Health Strip shows the real Doctor + computed summary | ✅ *(core-live)* |
| Chain status (live 40204 block) | ✅ |
| Composition Canvas drives live runs | ✅ *(STUDIO-10, core-live — real wasmtime dispatch on clip completion)* |
| Break-Glass real SecurityOfficer path | 🟡 (the UI + 72h affirmation are real; the live break-glass attestation rides the same `ApprovalQueue` wiring) |

## Quality bar (already met)

- **Both builds green, zero studio warnings.** Default (fast, modeled core) + `core-live`
  (real `citrate-agent-core`). 35 default / 40 core-live tests.
- **Parity-tested seams.** Every pure decision (SoD, quorum shape/count/decision, audit
  integrity) passes the *same* assertions under default and `core-live`.
- **e2e harness.** Headless gate→sign→resume→done→audit loop, asserted, both builds.
- **No dead code.** Every `#[allow(dead_code)]` removed or a precise `cfg_attr`; unwraps
  triaged — **9 non-test (5 default-build + 4 core-live), all provably safe** (the
  independent audit §6 corrected the prior undercount of 7; none is reachable with
  attacker- or network-controlled input).
- **Full Agentile trail.** 13 sprints, each with spec → tests → code → retro → journal → essay.
- **UI kit extracted** to `citrate-studio-ui-kit`, consumed by Studio + a second shell,
  pixel-identical *(STUDIO-13)*.
- **Visual regression gate** — 19 surfaces captured + golden-image diff *(STUDIO-11)*.

## What remains for `v1.0.0`

| Item | Kind | Gate |
|---|---|---|
| Signed `.cps` dispatch (drop the dev opt-in) | upstream | CIT-AGENT-3e packer |
| L0 first-prompt agent loop | upstream | CIT-AGENT-3 |
| Signed/notarized installers | ship | Apple + Windows code-signing certs (CI secrets) — **next** |
| External Tier-1 audit attestation | ship | the federation auditor (`AUDIT_TIER.md`) — **after** |

None of these is unknown or hidden — each is named with what gates it. The codebase is a
**hardened release candidate**: every capability that can be real in this environment is
real, parity-tested, and e2e-covered; the rest is functional UX work or genuinely external
(certs, a funded key, an auditor, the upstream agent loop/packer).
