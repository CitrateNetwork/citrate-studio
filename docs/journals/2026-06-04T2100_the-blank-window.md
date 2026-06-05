---
created: 2026-06-04T21:00:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-1
---

# Technical Journal — June 4, 2026

## The Blank Window: how a headless shell hid a layout bug for an hour

### Context

Mid-way through STUDIO-1 the whole app went blank. The minimal bring-up window had
rendered perfectly an hour earlier — fonts, two-zone layout, icons, a 109 KB PNG of
real content. Now, with the full shell wired, `take_snapshot()` returned a fully
transparent buffer. Same renderer, same fonts, same snapshot code. Nothing in the
build failed; no panic; exit 0.

Two failures were tangled together, and untangling them is the lesson.

### Failure one: the snapshot mechanism, not the UI

I assumed "blank PNG" meant "blank UI" and started bisecting the layout. Wrong first
move. The femtovg `take_snapshot()` re-renders the scene offscreen, and in this
headless shell — no window server attached to a terminal-launched GUI — there is no
presented surface for it to re-render from. It worked for the *minimal* tree and
failed for the *Flickable-heavy* one, which made it look like a UI regression. It
wasn't. `screencapture` of the screen showed only the desktop: the window never
mapped at all.

The fix was to stop fighting the surface and render without one:
`MinimalSoftwareWindow` + a custom `Platform`, drawing straight into a
`PremultipliedRgbaColor` buffer, un-premultiplied to RGBA8 for the PNG. No window,
no GPU, no event loop. That is the right tool for headless fidelity review, and I
should have reached for it immediately instead of after three renderer swaps.

A second, quieter trap: my Python pixel sampler ignored PNG row filters, so it
reported "0 non-transparent colors" on images that *did* have content. I "confirmed"
blankness with a broken instrument. The moment I viewed the PNG with a real decoder,
the UI was there. **Trusting a hand-rolled verifier over the actual artifact cost
more time than the bug.**

### Failure two: the real layout bug underneath

Once snapshots worked, a genuine bug remained: the body rendered empty under the
chrome. Two Slint semantics conspired:

1. **Elements default to `width: 100%; height: 100%` of their parent.** A `Chrome`
   component that `inherits Rectangle` does *not* derive its height from its inner
   `VerticalLayout`; with no explicit height it collapsed, and the layout gave it 0.
2. **A conditional `if` is not a stretchable layout slot.** `if advanced:
   HorizontalLayout { vertical-stretch: 1 }` as a direct child of a `VerticalLayout`
   did not receive the stretch — the body got 0 height.

The fix: give `Chrome` an explicit height, and wrap the conditional body in an
always-present `Rectangle { vertical-stretch: 1 }` that the `if` fills. Obvious in
hindsight; invisible while staring at a transparent buffer produced by a *different*
bug.

### The numbers

| Thing | Value |
|---|---|
| Renderer configs tried before the fix | 3 (femtovg live, software live, testing-backend) |
| Root cause of the blank PNG | no presented surface (headless), not the UI |
| Root cause of the empty body | default-100% sizing + conditional-not-stretched |
| Final verification | `MinimalSoftwareWindow` headless render, per-panel crops |
| Build warnings at close | 0 |

### Lesson

When a visual check fails, **separate "did it render?" from "did it render right?"
before touching the layout.** A blank artifact has at least two independent causes —
the capture path and the scene — and conflating them sends you bisecting the wrong
one. Pick a capture method that doesn't depend on a window surface
(`MinimalSoftwareWindow`), and verify with a *real* decoder, not a hand-rolled
sampler. Once the instrument is trustworthy, the actual layout bug is a five-minute
fix.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*Stack: Slint 1.16.1 · renderer-software · macOS 15.6 · headless shell.*
