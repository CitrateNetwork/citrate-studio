---
created: 2026-06-05T08:40:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: archived
sprint: STUDIO-12
---

# STUDIO-12 — Real chain anchoring (40204)

**Goal.** Anchor the audit-chain root on the live Citrate chain (40204) for real — closing
the last gated functional item in `COMPLETION_STATUS.md`.

**Done already (proven on-chain).**
- Generated a secp256k1 anchor key; address `0x6e5d…1929` (derivation verified against the
  runtime's known vector). Funded via `faucet.citrate.ai` (10 SALT).
- `core_bridge::anchor::anchor_root` uses the runtime's `RecorderClient` (EIP-155, chain
  40204) to write the root in a real tx. The `AgentDecisionRegistry` write methods are
  recorder-gated, so a non-rostered key anchors via a **self-tx carrying the root as
  calldata** — on-chain, readable, tamper-evident, gas-only.
- `core_bridge::audit::root` = the real `AuditChain` tip (`last_hash`).
- **Real anchor landed:** root `0x50d89f48…` → tx `0xb89dde7e…`, block 497881; verified the
  tx's calldata *is* the root via `eth_getTransactionByHash`.

**Remaining (this sprint).**
- UI affordance: a "Anchor audit root" action (Settings → Policy) that anchors off-thread
  and shows the tx hash; gated on `CITRATE_ANCHOR_KEY`.
- `COMPLETION_STATUS.md`: anchoring 🔒 → ✅.

**Plan.**

| # | Step | Status |
|---|---|---|
| 1 | `anchor::anchor_root` + `audit::root`; real anchor proven on 40204 | done |
| 2 | UI: "Anchor audit root" button (Settings → Policy) off-thread | done |
| 3 | COMPLETION_STATUS → anchoring ✅ | done |
| 4 | Default + core-live green | done |

**Daily updates.**

- 2026-06-05 — kickoff. Key funded via faucet; **real anchor landed** (tx `0xb89dde7e…`,
  block 497881, root verified in calldata). Wiring the UI.

**Decisions made.**

- **Self-tx anchor**, not the `AgentDecisionRegistry` contract method — the contract write
  methods are recorder-gated (a non-rostered key would revert); a self-tx with the root in
  calldata is a real, authorization-free anchor. A rostered key could use the contract.

**Exit criteria.**

- [x] A real audit root is anchored on chain 40204 (verified on-chain via `eth_getTransactionByHash`).
- [x] Settings → Policy "Anchor audit root" button anchors off-thread and shows the tx; gated on the key.
- [x] `COMPLETION_STATUS.md` reflects anchoring ✅ (and the remaining-items row removed).
- [x] Default + core-live green, zero studio warnings.

**Close note.**

STUDIO-12 closes the last gated functional item: the audit root is anchored in a **real
transaction on chain 40204**, proven on-chain (root `0x50d89f48…` → tx `0xb89dde7e…`, block
497881; the tx calldata *is* the root). The funded key (`0x6e5d…1929`, derivation verified
against the runtime's known vector) came self-serve from `faucet.citrate.ai`; the anchor uses
the runtime's `RecorderClient` (EIP-155) via a self-tx (the `AgentDecisionRegistry` write
methods are recorder-gated). A Settings → Policy button anchors on demand. The project's trust
chain is now real *and publicly verifiable* end to end: real key → real attestation → real
queue → real capsule → real audit chain → **real on-chain anchor**. The honest residue is
narrow: contract-method anchoring (needs recorder authorization), progress UX, keyring storage
for the anchor key in production. Retro + journal + essay alongside.
