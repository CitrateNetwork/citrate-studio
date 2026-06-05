---
created: 2026-06-04T23:45:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-3
---

# Build the Viewport, Earn the Swap

**A UI that never decides anything can have its entire backend replaced without
changing a line of itself. That property is not free — it is designed in from the first
screen, and STUDIO-3 is where the design either pays off or is exposed as a story we
told ourselves. It paid off. Here is why it did, and what it costs to keep earning it.**

## The thesis: the trust boundary is an architectural promise with a due date

From STUDIO-1, Citrate Studio was built on one claim: *the front end is a viewport and
an intent submitter; it never makes a policy decision.* Quorum, separation-of-duties,
audit integrity — all of it computed in a Rust "core" and handed to the UI as
ready-to-render state. In STUDIO-1 and -2 that core was *modeled*: hand-rolled rules in
`main.rs` standing in for `citrate-agent-core`, which is heavy (wasmtime, an
SSH-sourced wallet crate) and wasn't wired yet.

A modeled core is a promissory note. It says: *when the real core arrives, swapping it
in will be a change of wiring, not a rewrite of the UI.* STUDIO-3 is the note coming
due. Either the boundary was real and the swap is mechanical, or the boundary was
decoration and the "viewport" has been quietly deciding things all along. You only find
out when you try.

## How you make a swap mechanical instead of a rewrite

The mechanism is a **seam**: a small set of functions with one signature and two
implementations behind a feature flag. `policy::{is_conflict, can_approve,
quorum_satisfied}` and `audit_verify::verify`. The default build compiles a faithful
hand-rolled copy of the runtime's rules; `--features core-live` compiles a thin bridge
that calls the authoritative `citrate-agent-core`. Every caller in the UI computes
through the seam and cannot tell which is behind it.

Two properties make this more than a refactor:

- **The default stays shippable the whole way.** The real core is an *optional* dep, so
  the fast default build never pulls wasmtime. The app ships throughout the migration;
  the feature matures subsystem by subsystem until it's ready to become the default.
- **A parity test gates the merge under both builds.** `cargo test` and `cargo test
  --features core-live` run the *same* assertions. Green on both means the model and the
  real core agree. That single test is what turns "it compiles against the real core"
  into "it produces the same answer as the real core" — and it is the proof the boundary
  held, because if the UI had been secretly deciding anything, the model and the core
  would diverge and the test would catch it.

By the gate, four pure surfaces were the real core's: SoD (`hitl::is_conflict`), the
quorum shape and count (`Quorum::for_tier`), the quorum *decision*
(`Quorum::satisfied_by`), and audit integrity (`AuditChain::verify_integrity`). The
scrubber's tamper verdict became the runtime's own error string, verbatim. Zero `.slint`
files changed. The note was good.

## The honest part: a viewport-shaped boundary has a shape, and the shape is "pure"

The most valuable thing STUDIO-3 taught wasn't that the swap worked — it's *where* the
swap stops being mechanical. The original plan listed the async `ApprovalQueue`,
`CapsuleDispatch`, and a `DoctorReport` against a live context as STUDIO-3 steps. They
aren't the same kind of thing as the surfaces above.

SoD and quorum are **pure decisions**: give them inputs, they return a verdict, no
world required. You can wire those in-memory and parity-test them, because the model and
the core are both just functions. But dispatching a real `.cps` capsule, anchoring on a
chain, verifying ed25519 attestations against pinned payloads in an async queue — those
are **execution against an environment** that the studio demo doesn't have. There's
nothing to mirror in-memory, because the truth is in the filesystem, the chain, the
keys.

So the boundary that makes a UI swappable has a natural seam in it that the *plan*
didn't: pure policy on one side (wireable now, STUDIO-3), live execution on the other
(needs the environment, STUDIO-5/6). Refining the scope mid-sprint to respect that line
wasn't cutting corners — it was discovering the real shape of the boundary and naming
it. The alternative was a thin, faked "wiring" of the execution surfaces that would have
*looked* done and lied to the audit.

## Why this generalizes

Every system that intends to replace its own backend later — a prototype that will get a
real API, a mock service that will get a real one, a UI that fronts an evolving engine —
is making STUDIO-1's promise. The discipline that lets you keep it is the same:

1. **Put a seam where the decision lives**, not where it's convenient.
2. **Keep the stand-in shippable** so you're never blocked on the real thing.
3. **Gate the swap on a parity test** so "swapped" is a fact, not a hope.
4. **Respect the pure/impure line** — wire decisions early, defer execution to when the
   environment exists, and say which is which out loud.

A viewport that never decides is a strong architectural position, but it is a *position
you have to keep taking*, screen after screen, or it erodes into a UI that "just this
once" computes something the core should own. STUDIO-3 is the receipt that, three
sprints in, the position held — and the map of exactly how far "swap, not rewrite"
reaches before "live operation" begins.

---

*Essay by Claude (AI assistant), reflecting on the STUDIO-3 sprint with Larry.*
