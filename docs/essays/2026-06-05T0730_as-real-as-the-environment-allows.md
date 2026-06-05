---
created: 2026-06-05T07:30:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-10
---

# As Real as the Environment Allows

**There's a gap between "make it real" and "make it real *here*, with the capsules that
exist, and label the difference." The first is a slogan; the second is the actual craft.
STUDIO-10 wired the canvas to real capsule execution — and the interesting part isn't that
it dispatches, it's how it tells the truth about *what* it dispatches. Written so the next
"make it real" lands as precision, not pretense.**

## The thesis: realness has a resolution, and you should state it

"The canvas drives real CapsuleDispatch" can mean several things at different resolutions:

1. The canvas triggers a real wasmtime execution when a clip completes. *(true)*
2. The canvas runs the *recon scenario's* capsules for real. *(false — those don't exist)*
3. The output cards show real `ToolResult`s from the scenario. *(false, for the same reason)*

A lazy "make it real" picks the most impressive reading and ships it. An honest one states
the resolution: claim 1, not 2 or 3. STUDIO-10 dispatches the real `hello` smoke capsule on
each clip completion — a genuine wasmtime call returning a real string — and the output card
shows the modeled scenario result *plus* a badge reading
`wasmtime · hello("recon.snapshot") → "Hello, recon.snapshot"`. The badge is precise about
exactly which claim is true: a real capsule ran; it wasn't the recon capsule, because there
isn't one yet.

## Why the precise claim is worth more than the impressive one

It would have been easy to map each `recon.*` clip to *some* fleet capsule and let the card
imply the scenario was running for real. Nobody glancing at it would notice. But a
compliance product's entire value is that its surface tells the truth about what happened —
and an output card that implies "the recon pipeline executed" when a generic smoke capsule
ran is exactly the lie the product exists to prevent, committed in the product's own UI.

The precise badge costs a little impressiveness and buys total trustworthiness: a reviewer,
an auditor, or Larry can read it and know *exactly* what's real. When the recon capsules are
built and installed, the same wiring dispatches *them*, and the badge will say so — because
the badge reports what actually ran, not what the scenario implies. Honest plumbing
upgrades itself when reality does; pretense has to be unwound first.

## "As real as the environment allows" is a discipline, not an excuse

The phrase can be a dodge — "it's only a stub because the environment is limited." The
discipline turns it around: do the *most* real thing the environment supports, and mark the
boundary exactly. The environment here supports real wasmtime execution of a real capsule —
so do that, for real, on every completion. It does *not* yet supply recon capsules — so
don't claim them; label what you ran. The boundary isn't an apology; it's a specification of
what's true today and what unlocks tomorrow (the upstream packer building the recon fleet).

This is the same line every sprint of this project has walked: real SoD and quorum (not
modeled, under core-live), real keys, real attestations through the real queue, real chain
reads, real audit verification — and at each edge, a precise statement of where "real" stops
and what gates the next step. STUDIO-10 is that line applied to the canvas: real dispatch,
labelled to the exact resolution of its realness.

## The fixture that drives the real machine

A smaller honesty hides in `apply_seed`. It used to fake a completed run —
`playhead = 56; status = "done"`. The screenshot that proves real dispatch couldn't use a
faked run, because a faked run dispatches nothing. So the seed was rewritten to *drive the
real state machine*: tick, dispatch, approve the gate, tick, until done. Now the fixture
produces the state a real run produces, outputs and all.

The lesson generalizes past this one function: a test fixture or demo seed that hardcodes the
end state is a small lie that compounds — it can pass while the real path is broken, and it
can't show real side effects. A fixture that drives the actual code is slower to write and
worth it, because it can only show you the truth. The visual proof was real precisely because
the seed behind it was real.

## The rule, generalized

When you "make it real," state the resolution of its realness and label the boundary in the
UI itself. Do the most real thing the environment supports; don't imply the thing it doesn't.
Build provenance that reports what *actually* happened, so it upgrades itself when reality
catches up. And drive your fixtures through the real machine, so the proof can't be a
performance. Real is not a slogan you apply; it's a measurement you report — to the exact
resolution you can defend.

---

*Essay by Claude (AI assistant), reflecting on the STUDIO-10 sprint with Larry.*
