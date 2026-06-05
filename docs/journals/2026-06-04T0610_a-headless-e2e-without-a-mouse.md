---
created: 2026-06-04T06:10:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-8
---

# Technical Journal — June 4, 2026 (e2e)

## A headless e2e without a mouse

### The constraint that shapes the design

Slint compiles the UI to native code, and `slint::platform::set_platform` can be called
**once per process**. That single fact rules out the naive e2e shape — "spin up N UI
sessions, click around in each." You get one platform, one set of windows, for the whole
test binary. And there's no clean public API to synthesize a click at pixel (x, y) into a
compiled component and have it route to the right callback.

So the question becomes: what does an end-to-end test of *this* app actually need to
prove? Not that a mouse-down at (1040, 470) hits the Sign button — that's hit-testing,
which the layout engine owns. It needs to prove that **the operator loop works**: play
pauses at the gate, signing reaches quorum, resume approves and continues, the run
completes, and the ledger verifies. That's a sequence of *state transitions through real
code*, and you can drive it without a mouse.

### Extract the intent, share it between the UI and the test

The dock's Sign button does work — find the pending clip, derive the payload, sign through
the real path, record or reject. That work lived inside the `on_sign` callback closure,
unreachable from a test. The move was to pull it into a free function:

```rust
fn apply_sign(s: &mut RunState, role: &str) { /* the real sign intent */ }
fn apply_resume(s: &mut RunState) { /* the real resume intent */ }
```

Now the callback is a one-liner — `apply_sign(&mut st.borrow_mut(), &role)` — and the e2e
calls the *same* function. The test drives the exact code the UI runs, so it can't drift
from a parallel re-implementation. Extraction-for-testability and clarity-of-`main()`
turned out to be the same edit.

### Driving the loop

With a headless `MinimalSoftwareWindow` platform set once, the harness reads like the
operator's session:

```rust
let ui = StudioWindow::new()?;
let st = new_state_with(Roster::seed_demo());

st.borrow_mut().status = "running".into();
while st.borrow().status == "running" { st.borrow_mut().tick(); }   // → paused at gate
refresh(&ui, &st.borrow());
assert_eq!(st.borrow().pending.as_deref(), Some("c3"));
assert!(!ui.global::<AppState>().get_quorum_met());

apply_sign(&mut st.borrow_mut(), "Reviewer");
apply_sign(&mut st.borrow_mut(), "ComplianceOfficer");
refresh(&ui, &st.borrow());
assert!(ui.global::<AppState>().get_quorum_met());   // ← the real refresh→AppState bridge

apply_resume(&mut st.borrow_mut());
while st.borrow().status == "running" { st.borrow_mut().tick(); }   // → done
assert!(audit_verify::verify(&frames, false).ok);
```

The crucial assertion is `ui.global::<AppState>().get_quorum_met()` *after* `refresh` — it
proves the whole bridge from `RunState` through `refresh` into the `AppState` the UI binds
to, not just the core logic. That's the "end-to-end" part: state → intent → state →
render-model.

### It runs under both builds

Because `apply_sign` is `cfg`-routed (default: local sign + policy seam; `core-live`: the
real `ApprovalQueue` via `LiveQueue`), the same e2e test exercises the modeled path under
default and the *real async queue* under `core-live`. One test, two depths of reality.

### Lesson

When you can't simulate the input device, test the behavior the input invokes. Extract the
intent so the UI and the test share one code path, assert the state transitions *and* the
render-model bridge, and let the layout engine own hit-testing (covered by snapshot
review). An e2e test earns its name by covering the loop end to end — not by faking a
cursor.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*STUDIO-8 · headless e2e · apply_sign/apply_resume shared by UI + test · both builds.*
