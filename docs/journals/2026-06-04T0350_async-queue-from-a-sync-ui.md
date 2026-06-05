---
created: 2026-06-04T03:50:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-6
---

# Technical Journal — June 4, 2026 (live-ops)

## Driving an async approval queue from a synchronous UI — and dispatching a real capsule

### The async/sync impedance mismatch

The runtime's `ApprovalQueue::submit_for_action` is `async` and *parks* — it awaits a
oneshot resolver (or a 5-minute timeout) while signatures trickle in. Slint's event loop
is single-threaded and synchronous. You cannot `block_on` a future that's waiting for
human signatures on the UI thread; the window would freeze for minutes.

The shape that worked (`core_bridge::approvals::LiveQueue`):

1. **Hold a tokio runtime + an `Arc<ApprovalQueue>`.** The queue is shared; the runtime
   owns the parked future.
2. **`open()` spawns the submit and waits for registration.** The catch: the pending
   entry is inserted on the future's *first poll*, not at `spawn` time. So after
   `rt.spawn(async move { q.submit_for_action(...).await })`, I poll `payload_for(call_id)`
   in a short sleep loop until the entry materializes. Only then can a signature attach.
   This is the one inelegant seam — a "register, then await" split in the upstream API
   would remove it.
3. **`sign()` is fully synchronous.** `add_signature` locks a `std::Mutex` and does all
   the real work — `verify_attestation`, RM-G.1 roster authorization, SoD, and
   `Quorum::satisfied_by` — on the calling thread. No async needed. When the quorum is
   met it removes the entry and fires the resolver.
4. **Detect approval by absence.** After a successful `add_signature`, if
   `payload_for(call_id)` is now `None`, the entry resolved → quorum met. The UI needs
   exactly this boolean; it doesn't await the parked future (that's the agent loop's job).

So the only async surface is "park until resolved"; everything the UI touches is sync.

### The keys lined up by construction

The bridge to studio's own keys needed no adaptation. `Ed25519FileSurface::from_seed(secret, role)`
and studio's `signing::gen_keypair` both do `SigningKey::from_bytes(seed).verifying_key()`,
and both compute `signer_id = SHA-256(pubkey)`. So a studio `EnrolledSigner.secret` feeds
straight into the runtime's surface and the `StaticSignerRoster` (built from the enrolled
pubkeys) authorizes it. The end-to-end test — Reviewer + ComplianceOfficer keys satisfy a
real High gate — passed the moment it compiled.

### Dispatching a real capsule: the interface name bites

`CapsuleDispatch::load_from_dir(capsules/, …)` loaded the fleet fine, but the first
`call_raw("hello", "greeter", "greet", …)` failed: *missing interface "greeter"*. The
component model exports an interface under its **package-qualified** name, not the bare
identifier. The echo-chain test in agent-core had the clue — its `query` interface is
`citrate:echo-chain-capsule/query@0.1.0`. So hello's is
`citrate:hello-capsule/greeter@0.1.0`. With that, `call_raw(... that ..., "greet",
[Val::String("Aleia")])` returns `Val::String("Hello, Aleia")` — real wasmtime execution.

One gate to respect: the shipped capsules carry placeholder content-hashes, so `call_raw`
fails closed unless `CITRATE_ALLOW_UNVERIFIED_CAPSULES` is set. I set it in the dispatch
path (loudly), because the alternative — silently bypassing the integrity gate — is the
exact thing the gate exists to prevent.

### A real Doctor needs a real artifact

To get a *meaningful* `DoctorReport` rather than three "skipped"s, I gave the
`DoctorContext` a real `audit_chain_path`: write a genesis + two events to a temp file via
`FilesystemSink` + `AuditChain`, then point the checks at it. `AuditChainIntegrityCheck`
then actually walks it ("verified 3 records"); `AuditFilePermissionsCheck` actually stats
it ("owner-only 0600"). The report is real because the thing it inspects is real.

### Lesson

To drive an async, human-paced service from a sync UI: keep the parked future on a runtime
thread, make the mutating calls synchronous, and observe completion by state, not by
awaiting. And when an integration "just works" with no adapter, that's not luck — it's a
data representation chosen three sprints ago to match the system you're now connecting to.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*STUDIO-6 · LiveQueue · real hello-capsule dispatch · real Doctor vs a real chain.*
