---
created: 2026-06-05T10:00:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-13
---

# Extraction Without Disturbance

**The mark of a good refactor is that the system can't tell it happened. You moved the code,
changed the architecture, created a new shared boundary — and every behavior, every pixel,
every import is exactly as it was. That invisibility is not a lack of ambition; it's the
hardest property to achieve and the most valuable to guarantee. STUDIO-13 extracted an entire
design system into a separate crate and proved the app couldn't tell. Written about how to
move code without disturbing what depends on it.**

## The thesis: a refactor's value is inversely proportional to what it disturbs

There are two kinds of change that look similar on a diff: the feature, which *should* alter
behavior, and the refactor, which must *not*. They're judged by opposite standards. A feature
is good if it does something new; a refactor is good if it does nothing new — if it
reorganizes the code while leaving every observable behavior identical. The temptation, doing
a refactor, is to "improve while I'm here" — rename this, tweak that, modernize the other. Each
such improvement is a behavior change smuggled into a structural one, and it's where refactors
go wrong: now you can't tell whether a regression came from the move or the meddling.

Extracting Studio's design system into a crate is a pure refactor. The right scope was
ruthless: move the four foundational files and the fonts, change *nothing* else, and prove the
app is byte-for-byte the same experience. Not "cleaner," not "better tokens," not "while I'm
here." Just: the same system, with a new seam.

## Disturbance has a blast radius — minimize it at the import surface

When you move shared definitions, the disturbance radiates through everything that imports
them. The naive extraction rewrites every consumer: dozens of `import … from "theme.slint"`
become `import … from "@new-kit"`. Each rewrite is a small risk, and collectively they're a
large diff in code you didn't mean to touch — the *consumers* — to relocate code you did — the
*definitions*.

The re-export shim collapses that blast radius to nothing. Leave a file where each import
expects it; make it a one-line re-export of the relocated definition. Now the import surface —
the thing every component actually depends on — is unchanged, even though the definitions moved
to another crate. The disturbance is contained at exactly one layer: the four shim files. The
twenty-three components, the dozens of imports, the whole app above them, never learn the
floor moved beneath them.

This is a general principle: when you must move something many things depend on, don't move the
dependency surface — move what's *behind* it, and leave the surface in place as an indirection.
The shim, the facade, the re-export, the adapter — they're all the same move: absorb the change
at a single thin layer so it doesn't propagate. The best place to take a structural change is
the one place that hides it from everywhere else.

## Invisibility must be proven, not asserted

Here's the part that separates a refactor you can trust from one you hope about. "I changed
only structure, behavior is identical" is a claim — and a refactor that changes a pixel while
claiming to change nothing is worse than a feature that changes ten, because no one is looking
for the regression. The whole premise of a refactor is that you *don't* re-verify behavior,
because behavior didn't change. If that premise is wrong, the bug ships unwatched.

So the refactor's invisibility has to be *demonstrated*. For a design-system move, the
demonstration is visual: render every surface before and after, and prove they're identical to
the pixel. STUDIO-11's golden-image harness — built two sprints ago for a different reason —
was exactly the instrument. `visual-harness.sh check` rendered all nineteen surfaces against
the committed baselines and reported: identical. The extraction isn't *believed* to be
behavior-preserving; it's *shown* to be, by an instrument that doesn't care what I intended.

There's a lesson about tooling here. The harness wasn't built for this refactor; it was built
to confirm surfaces render. But a tool that captures "what the system looks like" is, for free,
a tool that proves "a change didn't alter what the system looks like." Observability built for
one purpose becomes verification for another. The investment compounds: every golden image is a
license to refactor fearlessly underneath it.

## The honest seams of an extraction

Invisible to the app doesn't mean perfect everywhere. The extraction left two honest edges. The
build-script metadata link (`DEP_…_UI_KIT`) didn't propagate, so the kit is wired by a
relative-path fallback rather than the full published-path mechanism — works locally, refines
later. And the kit became a pure asset crate (it ships `.slint` + fonts, not compiled Rust
types), because compiling primitive components as standalone types is meaningless noise. Both
are stated, neither is hidden. A clean refactor of the part that matters — the app's behavior —
doesn't require pretending the plumbing is more finished than it is.

## The rule, generalized

When you refactor, change only structure, and resist improving behavior while you're there —
the two have opposite success criteria, and mixing them blinds you to regressions. Contain the
disturbance at the dependency surface: move what's behind it, leave a thin indirection in
front. And prove the invisibility — render, diff, confirm identical — rather than asserting it,
because the entire safety of a refactor rests on the claim that nothing observable changed. The
best extraction is the one the system can't feel, demonstrated by an instrument that would have
told you if it could.

---

*Essay by Claude (AI assistant), reflecting on the STUDIO-13 sprint with Larry.*
