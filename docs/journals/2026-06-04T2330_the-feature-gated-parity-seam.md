---
created: 2026-06-04T23:30:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-3
---

# Technical Journal — June 4, 2026

## The feature-gated parity seam: migrating a heavy real dependency without stopping the world

### Context

STUDIO-3 had to replace a modeled, in-process "core" with the real
`citrate-agent-core` — which drags in `wasmtime 45`, `cranelift`, and an
SSH-sourced wallet crate, a ~4-minute build. The naive options were both bad:
swap it in wholesale (a long, risky, all-or-nothing change that breaks the
shipping app until every subsystem is ported), or keep faking it (and never
actually prove the integration). The pattern that worked is worth writing down,
because the remaining surfaces — and the next native app — will reuse it.

### The shape

Three moving parts:

1. **An optional, feature-gated dependency.** `citrate-agent-core` is
   `optional = true` behind a `core-live` feature. The *default* build never
   compiles it — stays fast, stays shippable. The feature is the integration
   surface, not the trunk.

2. **A seam with two implementations behind one signature.** A `policy` (and
   later `audit_verify`) module exposes plain functions. `#[cfg(not(core-live))]`
   is a faithful hand-rolled copy of the runtime's rule; `#[cfg(core-live)]` is
   `pub use` of a bridge that calls the real core. Callers — `quorum_met`,
   `approval_roster`, the scrubber verdict — compute through the seam and never
   know which is compiled.

3. **A parity test that must pass under *both* feature builds.** This is the
   keystone. `cargo test` and `cargo test --features core-live` run the *same*
   assertions; green on both means the modeled stand-in and the authoritative
   core agree. The test is what makes the default build trustworthy and the swap
   provable in the same stroke.

### Why it's better than the alternatives

- **The app never stops shipping.** Default build is green and fast the entire
  migration; `core-live` matures subsystem by subsystem until it's ready to
  become default.
- **The integration is proven, not asserted.** "It compiles against the real
  core" and "it produces the same answer as the real core" are both CI facts,
  not hopes.
- **The boundary stays honest.** The seam made it obvious which surfaces are
  *pure decisions* (wireable in-memory: SoD, quorum, integrity — done) versus
  *execution against an environment* (async approvals, capsule dispatch, chain
  anchoring — which have nothing to mirror in-memory, so they belong to a
  live-operation sprint, not this one). The pattern surfaces the right scope.

### The one caveat

Parity tests prove agreement *on the cases they exercise*, not total
correctness of the stand-in. The real core is the source of truth; the modeled
default is a fast-build convenience the tests keep honest only where they look.
State that plainly so "parity" isn't over-read.

### Lesson

To wire a heavy, real dependency into a shipping app: make it optional behind a
feature, put a two-implementation seam behind one signature, and gate the merge
on a parity test that runs under both builds. You get a fast default, a provable
real path, and — for free — a clean line between the decisions you can wire now
and the execution you must wire later.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*STUDIO-3 · `policy` + `audit_verify` seams · 14 tests green on both feature builds.*
