---
created: 2026-06-04T04:00:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-6
---

# The Bridge Built Three Sprints Early

**Integration is usually translation: two systems that grew up apart, joined by a layer of
adapters that map one's idea of a thing onto the other's. The cheapest integration is the
one where there's nothing to translate — where a value you computed for your own reasons
turns out to be byte-identical to what the other system expects. That doesn't happen by
luck. It happens when you choose your representations, early, to match the system you know
you'll have to meet later. STUDIO-6 is the moment that bet paid out. Written so the next
builder front-loads the same kind of decision.**

## The thesis: design for the integration you can see coming

When STUDIO-4 generated ed25519 signer keys, it had a choice for how to derive a signer's
identity. It chose `signer_id = SHA-256(pubkey)` — not because anything in STUDIO-4 needed
that exact formula, but because the runtime's `hitl::signing::signer_id_from_pubkey`
computes identity that way, and STUDIO-4 knew STUDIO-6 would eventually have to hand
signatures to the runtime's `ApprovalQueue`. It was a small act of foresight: spend a line
now matching a representation, to save an adapter later.

The line came due this sprint. To feed a studio signature into the real async approval
queue, the code is:

```rust
let surface = Ed25519FileSurface::from_seed(enrolled.secret, role);
let sig = surface.sign(&payload).into();
queue.add_signature(call_id, sig)?;
```

There is no mapping layer. The studio secret produces the same pubkey the runtime expects;
`SHA-256(pubkey)` yields the same `signer_id`; the `StaticSignerRoster` built from the
enrolled pubkeys authorizes it; `verify_attestation` accepts it; `Quorum::satisfied_by`
counts it. The end-to-end test — two enrolled keys satisfying a real High gate through the
real queue — passed the instant it compiled, because there was nothing *to* get wrong in
between. The integration wasn't a translation. It was a connection.

## Why this is the cheaper path, and the harder discipline

The tempting alternative is always to defer the matching. Generate keys however is
convenient now; when integration day comes, write the adapter that converts your format to
theirs. This feels like good separation of concerns. It is usually a trap, for three
reasons.

First, adapters are where the subtle bugs live — an endianness flip, a hash over the wrong
bytes, a role enum that *almost* lines up. Every adapter is a small translation you have to
keep correct as both sides evolve.

Second, the adapter hides the coupling instead of removing it. You're still coupled to the
runtime's identity scheme; you've just buried the coupling in a conversion function where
it's easy to forget and easy to break.

Third — and this is the real cost — the adapter defers the *proof*. As long as there's a
translation layer, "do studio's keys actually work against the real queue?" stays an open
question until integration day. Match the representation up front and the question is
answered by construction; the only thing left to test is that you wired the connection, not
that the two worlds agree.

The discipline this demands is foresight under uncertainty: at STUDIO-4 time, commit to the
*downstream* system's representation before you've built the thing that consumes it. That
requires having read the runtime's signing code three sprints before you needed it, and
trusting that the integration would come. It's a bet. STUDIO-6 is the receipt that it was
the right one.

## "Real all the way down" reaches the bottom

There's a second thread that closes here. Across STUDIO-3 through 6 the recurring rule was:
make each primitive real, or stub it so it says so. STUDIO-3 made policy real. STUDIO-4
made the *keys* real. STUDIO-5 made *setup* real. STUDIO-6 makes the *execution* real — and
because each layer was real, the layers compose without seams:

- a real ed25519 key (STUDIO-4)
- signs a real attestation that the real async `ApprovalQueue` verifies and counts
  (STUDIO-6),
- gating a real capsule that wasmtime actually executes — `"Hello, Aleia"` from a real WASM
  component (STUDIO-6),
- against a real chain whose id (40204) and block height come from `rpc.citrate.ai`
  (STUDIO-6),
- inspected by a real `DoctorReport` whose integrity check walks a real audit chain
  (STUDIO-6).

None of these is modeled. The chain of trust the product *claims* — a real key approves a
real action that runs real code recorded on a real ledger — is, end to end, a chain of real
things. That's only possible because nobody faked a link. A single modeled primitive
anywhere in that chain would have made the whole composition theatre.

## The honest edges, named

Foresight doesn't mean finished. The live dock still decides quorum through STUDIO-3's
policy seam — the real queue is *proven and available*, one wiring step from being the
running UI's path. Chain *writes* wait on a funded signer. Signed-capsule dispatch waits on
the upstream packer (CIT-AGENT-3e), so today's real execution rides a loudly-logged dev
opt-in. The L0 agent loop is upstream too. These are integration and dependency, not
unknowns — and naming them is the same discipline as matching the representation: be exact
about what is true.

## The rule, generalized

When you can see an integration coming — a real backend behind your prototype, a runtime
behind your UI, a protocol you'll have to speak — spend the cheap line now to match its
representation, instead of the expensive adapter later. Read the downstream system before
you need it. Choose your `signer_id`, your hash inputs, your enum names, your wire format,
to be *the same value* the other side computes. Then integration day isn't a translation
project with a debugging tail — it's a connection that works the moment it compiles,
because the two systems were never speaking different languages. You just had the foresight
to teach yours theirs, three sprints early.

---

*Essay by Claude (AI assistant), reflecting on the STUDIO-6 sprint with Larry.*
