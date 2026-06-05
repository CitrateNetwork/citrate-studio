---
created: 2026-06-05T08:10:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-11
---

# The Right Tool for the Substrate

**When someone asks for a specific tool, they're usually naming an outcome, not a
requirement. "Run Playwright" means "give me confidence the UI works, visually, like the
mature web harnesses do." The honest engineer separates the outcome from the tool, checks
whether the tool fits the substrate, and — when it doesn't — delivers the outcome with the
tool that does, and says why. Written so the next "use X" gets the intent, not a
cargo-culted X that can't work.**

## The thesis: a tool that doesn't fit the substrate can't deliver the outcome

Playwright is superb. It is also, specifically, a *browser* automator: it speaks the
DevTools/WebDriver protocol to a Chromium, WebKit, or Firefox process and manipulates a DOM.
Citrate Studio is a native Rust + Slint application that rasterizes its UI to a pixel buffer.
There is no browser, no DOM, no protocol endpoint — there is nothing for Playwright to
*attach to*. Pointing Playwright at the native binary isn't a hard integration; it's a
category error. The tool's entire mechanism assumes a substrate this app doesn't have.

So the request "a strong harness that runs Playwright" contains a false premise for this
target. The wrong response is to honor the literal words and build a Playwright project that
silently can't run. The right response is to honor the *intent* — strong visual confidence in
every surface — and notice that the intent and the tool have come apart.

## Separate the outcome from the tool

What does "Playwright-grade visual e2e" actually deliver, as an outcome? Three things: it
reaches every meaningful UI state, it captures each as an image, and it fails when an image
drifts from a known-good baseline. None of those three is intrinsically about a browser. They
are about *driving a UI to states and asserting its rendering*. A browser is just the
substrate Playwright happens to drive.

Once you state the outcome substrate-independently, the native equivalent is obvious: drive
the Slint app to each state (it's already addressable by env seed), render each headlessly to
a PNG (the software renderer does this), and diff against committed golden images. Same three
guarantees — reach, capture, regression-gate — on the substrate this app actually has. The
harness isn't a downgrade from Playwright; it's *Playwright's outcome*, implemented for a
pixel buffer instead of a DOM.

## Why saying "no, and here's what instead" is the high-integrity move

It would have been easy, and superficially responsive, to scaffold a Playwright project,
write some specs, and hand it over — it *looks* like what was asked. But it would fail the
moment anyone ran it, and worse, it would imply a kind of coverage that doesn't exist. On a
project whose entire ethos is "real, not faked; don't minimize the architecture; answer the
hard questions" — a harness that can't run is exactly the faked artifact the ethos exists to
prevent, dressed as compliance with a request.

The high-integrity move is to answer the hard question directly: *Playwright can't drive this
app, here's why, here's the equivalent-strength harness that can, and here's where Playwright
genuinely belongs.* That last clause matters too — Playwright isn't wrong in general, it's
wrong for *this substrate*. It is exactly right for the web prototype this app was ported
from, and for a future web re-skin. So the deliverable includes a documented Playwright
scaffold pointed at those, honestly labelled as "for when a web surface exists." The tool gets
placed where it fits, not discarded and not misapplied.

## The deeper pattern: build the API before you need it

There's a quieter lesson in how cheap the native harness was to build. It captures 19 surfaces
in one small script — because every earlier sprint, adding a headless screenshot for its own
verification, had been incrementally building an env-addressable test API without calling it
that. `CITRATE_STUDIO_SEED`, `OVERLAY`, `SECTION`, `TAMPER`, `SIGNEDIN` — each was added for a
screenshot, and together they're the harness's whole interface. The visual e2e was latent in
the codebase, waiting to be enumerated.

That's the reward for a habit: a headless render entrypoint plus state-by-environment is a
test API whether or not you've named it one. Build for observability as you go, and the
comprehensive harness is a script away. The "strong harness" Larry asked for didn't need to be
constructed so much as *recognized* — it was already there, in the seams the project had been
leaving for itself.

## The rule, generalized

When asked for a specific tool, extract the outcome it's a proxy for. Check the tool against
the substrate. If it fits, use it. If it doesn't, deliver the same outcome with the tool that
fits, explain the mismatch plainly, and place the requested tool where it *does* apply rather
than discarding or misapplying it. And build your systems to be driven from the outside as you
go — so the day someone asks for a strong harness, you're enumerating an API you already have,
not inventing one you wish you did.

---

*Essay by Claude (AI assistant), reflecting on the STUDIO-11 sprint with Larry.*
