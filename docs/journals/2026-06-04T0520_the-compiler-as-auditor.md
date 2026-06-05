---
created: 2026-06-04T05:20:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-7
---

# Technical Journal — June 4, 2026 (hardening)

## The compiler as auditor, and the cfg_attr distinction

### Turning `allow(dead_code)` off to see the truth

The codebase had five `#[allow(dead_code)]` at the module level — one per non-trivial
module. Each was a small deferred decision: "something in here isn't used yet; don't warn
me." The problem with a blanket module allow is that it masks *new* dead code forever, not
just the intended item.

The audit technique was simply to remove them and let the compiler talk:

```
$ perl -0pi -e 's/^#\[allow\(dead_code\)\]\n//mg' src/main.rs
$ cargo build 2>&1 | grep "never"
warning: function `signer_id_from_pubkey` is never used
warning: struct `RosterEntry` is never constructed
warning: function `enroll` is never used
warning: field `sub` is never read
...
```

The compiler is a better auditor than any grep: it can't be fooled about what's actually
called. Every name it printed was either (a) genuinely dead — STUDIO-2 scaffold that
STUDIO-4 superseded, delete it — or (b) cfg-conditional — used in one build, not another.

### Dead code vs cfg-conditional code

The two categories need opposite treatments, and conflating them is how you either delete
something you need or suppress something that's rotting.

**Genuinely dead** (`signer_id_from_pubkey`, `RosterEntry`, `enroll`, `Claims.sub`): the
final design replaced it. Delete it. Smaller, truer code.

**cfg-conditional** (`sign_for`, `data::doctor`): used in the *default* binary, unused in
the *core-live* binary (which signs through `LiveQueue` and shows the real doctor). This
isn't dead — it's alive in a different cfg. The precise annotation:

```rust
#[cfg_attr(feature = "core-live", allow(dead_code))]
pub fn sign_for(...) { ... }
```

That reads as "allow dead_code *only* under core-live" — exactly the build where it's
unused. Under default it stays warned (so real rot would surface), and tests still call it
in both. A blanket `#[allow(dead_code)]` would have hidden the truth in all builds; the
`cfg_attr` tells it.

### Wire it, don't allow it

A third category: the STUDIO-6 dispatch + doctor bridges were *proven by tests* but unused
in the binary. The lazy fix is `allow(dead_code)`. The honest fix is to wire them into the
UI — a "Run smoke capsule" button that actually dispatches `hello` through wasmtime, and a
Health Report fed by the real `DoctorReport` with a *computed* summary (count the pass/warn
rows, color the LED) instead of the hardcoded "10 pass · 1 warn". Dead code became a
feature. An `allow` would have frozen a capability at "tested but unreachable."

### The trap: an incremental cache that lies

Mid-sprint, after a flurry of `touch src/*.rs` + `cargo build` + `cargo test`, the linker
failed:

```
Undefined symbols for architecture arm64:
  "_anon.56051a40….llvm.12701613…", referenced from:
    i_slint_core::platform::Platform::set_event_loop_quit_on_last_window_closed
```

This looks like a code error. It isn't — `_anon.*.llvm.*` undefined symbols are stale
incremental object files referencing LLVM-internal names that no longer exist after partial
recompiles. `rm -rf target/debug/incremental` and a clean rebuild fixed it instantly. The
lesson: don't `touch` files to force rebuilds (it's what poisons the cache), and when you
see LLVM anon-symbol link errors, suspect the cache before the source.

### Lesson

Let the compiler audit dead code — turn off the `allow` and read the warnings. Then sort
each finding: delete the truly dead, `cfg_attr`-annotate the cfg-conditional, and *wire*
the merely-unreached. A blanket `allow` answers all three with "ignore," which is why it
accumulates debt. And treat a flurry of partial rebuilds as a cache hazard, not a
correctness signal.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*STUDIO-7 · 5 allows removed · dead code deleted, cfg-conditional annotated, bridges wired.*
