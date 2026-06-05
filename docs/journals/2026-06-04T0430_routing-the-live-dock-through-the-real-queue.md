---
created: 2026-06-04T04:30:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-6
---

# Technical Journal — June 4, 2026 (dock rewiring)

## Routing the live dock through the real ApprovalQueue

### The change

STUDIO-6 proved the real async `ApprovalQueue` works end-to-end, but the *running*
dock still decided quorum through STUDIO-3's `policy::quorum_satisfied` seam. This
commit makes the live UI's Sign button drive the real queue — under `core-live` —
while the default build keeps the lightweight path.

### Conditional state without a forked struct

`RunState` now carries the queue, but only under `core-live`:

```rust
#[cfg(feature = "core-live")]
live: Option<core_bridge::approvals::LiveQueue>,
#[cfg(feature = "core-live")]
live_open: Option<String>,
sign_error: String,   // always present — the display surface is feature-agnostic
```

`cfg`-gated struct fields are cleaner than two whole structs: the init in
`new_state_with` carries the same `#[cfg]`, and every other field is shared. The one
rule is that anything the *UI* reads (`sign_error`) stays unconditional, so the model
push in `refresh()` doesn't need a `cfg`. The view never learns which core it's talking
to — same discipline as every seam before it.

### The borrow choreography in `live_sign`

`LiveQueue::sign` needs the signer's secret (from `s.roster`) and a mutable `s.live`.
Touch both naively and the borrow checker objects. The fix is to *extract everything
owned first*, dropping the `s.roster` borrow, before reaching for `s.live`:

```rust
let pairs: Vec<([u8;32], String)> = s.roster.signers.iter().map(...).collect();
let signer = s.roster.signer_for(role).cloned();      // owned
let secret = signer.secret;                            // owned
// ...s.roster borrow is now done...
if s.live.is_none() { s.live = Some(LiveQueue::new(&pairs)); }   // mut s.live
```

A free `fn live_sign(s: &mut RunState, ...)` reads better than a method here, because the
sequence is "gather, then mutate one field, then mutate another," and that's clearer as a
flat function than threaded through `&mut self`.

### The inversion that matters: record only if accepted

The old default path signs locally and *always* records the signature, trusting the
policy seam to decide quorum. The queue path inverts it: `add_signature` is the
authority, so the UI records a signature **only if the runtime accepts it**:

```rust
match lq.sign(&cid, secret, role) {
    Ok(_met) => s.signatures.push((role, name, "Slint".into())),
    Err(e)   => s.sign_error = e,   // verify failed / not authorized / SoD / dup
}
```

This is the whole point of routing through the real queue: a signature the runtime would
reject never appears in the UI as accepted. The dock already disables SoD/unauthorized
rows before a click, so `sign_error` is defensive — but defense-in-depth is exactly what
you want on the approval path. The two builds converge on the same *answer* (both use
`Quorum::satisfied_by`); they differ in *who enforces the attestation*, and under
`core-live` that's the runtime.

### Lesson

To make a running UI use a real backend it was designed for, you don't rewrite the flow —
you `cfg`-gate the state, extract-then-mutate to satisfy the borrow checker, and invert
"record then decide" into "decide then record" so the authority is the backend, not the
view. The seams the earlier sprints built meant this was a contained change, not a
refactor.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*STUDIO-6 · live dock → real ApprovalQueue · default and core-live converge on the same verdict.*
