---
created: 2026-06-05T07:25:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-10
---

# Technical Journal — June 5, 2026

## Dispatching a capsule the moment a clip finishes

### The trigger: crossing the end boundary

The playback `tick()` already detected clips crossing their *start* (medium queue) and
*end* (auto-feed). The dispatch trigger is the general case of the latter — any clip whose
end falls inside this tick's `[prev, next)` window just finished:

```rust
self.just_completed.clear();
for c in &base {
    let ce = (c.start + c.dur) as f32;
    if prev < ce && next >= ce {
        self.just_completed.push(c.id.to_string());
    }
}
```

`tick()` stays pure (it only records ids) — the I/O lives in a separate
`dispatch_completions(s)` the timer calls right after. That keeps the state machine
unit-testable and the side effect isolated.

### Lazy-load the dispatcher, dispatch per completion

```rust
#[cfg(feature = "core-live")]
fn dispatch_completions(s: &mut RunState) {
    if s.just_completed.is_empty() { return; }
    if s.dispatch.is_none() {
        s.dispatch = LiveDispatch::load(&capsules_dir()).ok();   // wasmtime engine, once
    }
    for id in std::mem::take(&mut s.just_completed) {
        let name = s.clip(&id).map(|c| c.name.to_string()).unwrap_or_default();
        let result = s.dispatch.as_ref()
            .map(|d| d.greet(&name))           // real wasmtime call_raw
            .transpose().ok().flatten()
            .map(|r| format!("wasmtime · hello(\"{name}\") → \"{r}\""))
            .unwrap_or_default();
        s.outputs.insert(id, result);
    }
}
```

The borrow dance: take the ids (releasing the field), then per id compute `name` (immutable
borrow, dropped), compute `result` (immutable borrow of `s.dispatch`, dropped), then
`s.outputs.insert` (mutable). Sequential, no overlap.

### The result flows through the existing model

`refresh` already rebuilt the clips with `dimmed` applied; one more line attaches the real
output:

```rust
if let Some(out) = st.outputs.get(c.id.as_str()) { c.live_output = out.clone().into(); }
```

and the `OutputCard` shows a green `wasmtime` badge when `live-output != ""`. No new model
plumbing — the dispatch result rides the clip struct the card already binds to.

### `apply_seed` stopped hardcoding

The screenshot wanted a completed run with real outputs, but the old `apply_seed` set
`playhead = 56; status = "done"` directly — no ticks, no dispatch, empty outputs. The fix
was to *drive the real machine*:

```rust
"done" => { s.status = "running".into();
    for _ in 0..600 { match s.status.as_str() {
        "running" => {},
        "paused"  => { approve_gate(s); },
        _ => break,
    } s.tick(); dispatch_completions(s); } }
```

Now the seed produces the same state a real run would, outputs included. A test fixture that
drives the real code is worth more than one that asserts a hardcoded end state — and it's
what made the visual proof real.

### Lesson

Keep the state machine pure and put the side effect next to it, not inside it. Trigger work
on the boundary crossing you already compute. Ride the existing model to the view instead of
adding plumbing. And make your seeds drive the real machine — a fixture that lies is a test
that lies.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*STUDIO-10 · dispatch on clip completion · pure tick + isolated I/O · honest seeds.*
