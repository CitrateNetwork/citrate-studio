---
created: 2026-06-04T06:20:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-8
---

# Testing the Loop, Not the Clicks

**An end-to-end test is supposed to give you confidence the whole system works. The
mistake is to think "whole system" means "including the mouse." It doesn't. The thing
worth proving is that the *behavior* survives the trip from intent to state to render —
and you can prove that without synthesizing a single input event. Written so the next
e2e harness aims at the loop, not the cursor.**

## The thesis: an e2e test asserts behavior, not input mechanics

There's a seductive picture of end-to-end testing: a robot that moves the cursor, clicks
the button, types in the field, and reads the screen — exactly as a user would. It feels
maximally real. But it conflates two very different things: the *behavior* of your system,
and the *mechanism* by which a human triggers it. Your code owns the first. The toolkit,
the OS, and the layout engine own the second. An e2e test that spends its effort
simulating clicks is testing the parts you didn't write, and it's brittle precisely there
— a layout shift moves the button, the "click at (1040, 470)" misses, and the test fails
for a reason that has nothing to do with whether the system works.

What you actually want to know is: when the operator expresses the intent "sign this
gate," does the system reach quorum, approve, continue, complete, and leave a verifiable
ledger? That is a chain of state transitions through your code. The cursor is just how the
intent gets expressed. Test the chain.

## The set-platform-once constraint was a gift

Slint can set its platform once per process and offers no clean way to inject synthetic
click events into a compiled component. My first reaction was "that makes e2e hard." It
didn't — it removed a temptation. With no way to fake a mouse, the only path was to drive
the *intents* the mouse would invoke and assert the resulting state. The constraint forced
the test toward the thing worth testing.

This happens often: a limitation in a tool, taken seriously, points you at a better design
than you'd have chosen with full freedom. Given a click-simulator, I'd probably have built
one and called it thorough. Denied one, I built something better — a test that drives the
real intent code and asserts the real render-model bridge — and it's both simpler and more
robust.

## Extract the intent, and the UI and the test stop being able to disagree

The enabling move was small: pull the Sign and Resume logic out of the callback closures
into `apply_sign`/`apply_resume`, free functions the callbacks now delegate to. The point
isn't tidiness. It's that the e2e test and the production UI now invoke the *same function*.
There is no parallel test-only re-implementation that can drift from the real one — the
classic way e2e tests rot into lies. When the UI's behavior and the test's expectation are
literally the same code, the test can't pass while the app is broken in that path.

This is the deeper principle behind "test the loop, not the clicks": arrange your code so
the testable seam is the *same seam the UI uses*. The callback should be a thin delegator
to an intent function; the intent function is what both the user (via the callback) and the
test drive. Then the test isn't an approximation of the user — it's the user's exact
action, minus the cursor.

## The render-model bridge is the "end" of end-to-end

It would be easy to stop at the core logic — drive the state machine, assert the
`RunState`. But that's not end-to-end; it's a unit test of the core. The "end" is the UI's
view of the world. So the load-bearing assertion in this harness is the one *after*
`refresh`:

```rust
refresh(&ui, &st.borrow());
assert!(ui.global::<AppState>().get_quorum_met());
```

That proves the whole path from `RunState`, through the `refresh` that computes the
derived UI state, into the `AppState` global the Slint components actually bind to. If
`refresh` forgot to push `quorum_met`, or computed it wrong, the real UI would render
wrong and this assertion would catch it. Asserting the render-model — not just the core,
and not the pixels — is what makes it end-to-end. The pixels are the layout engine's job;
the snapshot review covers those.

## Honesty about the last mile

This harness does not prove that a click at a particular coordinate hits the Sign button.
That hit-testing is real, and it's unverified by automation here — covered instead by
reviewing headless snapshots. Saying so is part of the discipline: an e2e test that
*implies* it covers input hit-testing when it doesn't is worse than one that's clear about
its boundary. The boundary is: behavior and render-model, automated; pixel hit-testing,
by-eye. That's an honest, defensible line — and it puts the automation where the bugs that
matter actually live.

## The rule, generalized

When you build an end-to-end test, aim it at the behavior, not the input device. Extract
the intent so the UI and the test share one code path. Drive that path and assert the state
transitions *and* the render-model the UI binds to — that's what makes it "end to end."
Let the layout engine own hit-testing, and say plainly that you've drawn the line there. A
good e2e test is a high-fidelity recording of the user's *intent and its consequences*, not
a puppet show of their hands.

---

*Essay by Claude (AI assistant), reflecting on the STUDIO-8 sprint with Larry.*
