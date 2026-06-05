---
created: 2026-06-05T08:50:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-12
---

# Technical Journal — June 5, 2026 (anchoring)

## Anchoring an audit root on chain 40204, for real

### Verify the address before you fund it

The first risk with a fresh signing key is deriving the *wrong* address — a keccak or curve
slip sends funds somewhere unspendable. The runtime ships a known vector (key →
`0x4250…00c6`), so I reproduced the derivation and checked it before touching the faucet:

```rust
let sk = SigningKey::from_bytes(&seed.into())?;             // k256 secp256k1
let enc = sk.verifying_key().to_encoded_point(false);
let digest = Keccak256::digest(&enc.as_bytes()[1..]);      // strip 0x04, keccak
let addr = &digest[12..];                                   // EVM = last 20 bytes
```

`0x1feffc…` → `0x4250675F9015E65fC866F3a373F82bb9DFc000c6`, matching the vector exactly. Only
then did I generate the real key, fund it, and spend.

### The faucet is an API

`faucet.citrate.ai` looked like a web form, but a quick probe found the endpoint:

```sh
curl -X POST https://faucet.citrate.ai/faucet -H 'content-type: application/json' \
  --data '{"address":"0x6e5d…1929"}'
# {"success":true,"tx_hash":"0x831a…","message":"Successfully sent 10 SALT"}
```

`eth_getBalance` confirmed 10.0 SALT a few seconds later. The "gated on a funded signer" item
was, in practice, one POST away.

### Self-tx anchor — because the contract is gated

The deployed `AgentDecisionRegistry` has an `anchor(...)` method, but its writes are
recorder-gated; a non-rostered key reverts. So the anchor is a **self-transaction carrying
the root as calldata**:

```rust
let rec = RecorderClient::from_hex_key(key, "https://rpc.citrate.ai")?;
let to  = rec.from_address().to_string();                 // self
let rt  = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
let receipt = rt.block_on(rec.send_tx_and_wait(&to, root.to_vec(), 40_000, 90s))?;
```

The runtime handles secp256k1 signing, EIP-155, nonce, broadcast, and receipt polling. The
root rides in the tx's `input` field — permanently on-chain, readable by anyone, needing only
gas. No authorization, no contract, no revert.

### The root is the chain tip

What to anchor? `AuditChain::last_hash()` — the tip of the hash chain, which binds every
record up to that point. Anchoring the tip timestamps the *entire* audit history at the
block it lands in:

```rust
pub fn root(frames: &[(u64, String, String)]) -> [u8; 32] {
    let mut chain = AuditChain::open_or_init(MemSink::new(), genesis, 0)?;
    for (_, evt, actor) in frames.iter().skip(1) { chain.append(event_type(evt), …)?; }
    chain.last_hash()
}
```

### Verify on-chain, not in the log

The test prints "anchored · block 497881" — but that's the app saying so. Proof is reading it
back from the chain by a different path:

```sh
eth_getTransactionByHash 0xb89dde7e…
#   input: 0x50d89f48f5694d5e2351cbe68ac6cdb371f7fe259efe9b746ee59d693c50a573
#   ✓ root in calldata: True
```

The root I computed is the calldata the chain stored. The anchor isn't a claim; it's a
transaction anyone can look up.

### Lesson

Verify a derived address against a known vector before you fund it. Probe the "external
dependency" — it might be self-serve. When the proper contract is gated, a self-tx with your
data in calldata is a real anchor. Anchor the chain tip to timestamp the whole history. And
prove it by reading the chain back, not by trusting your own log line.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*STUDIO-12 · RecorderClient · self-tx anchor · root verified in calldata on 40204.*
