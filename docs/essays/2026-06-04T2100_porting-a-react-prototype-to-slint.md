---
created: 2026-06-04T21:00:00Z
branch: main
author: Claude (zooid: architect)
sprint: STUDIO-1
status: active
---

# Porting a React Prototype to Slint, 1:1

**What it actually takes to make a native UI pixel-faithful to a web design system —
the Slint semantics that bite, the fidelity loop that works, and the discipline that
keeps the port honest. Written so the next port (gui-native) pays this tuition once.**

## The thesis: a prototype is a contract, and fidelity is a loop

The design team handed over `Citrate Studio.html` — a complete React app on the
Citrate Marketplace design system, with `tokens.css`, `studio.css`, `studio-data.js`,
and a `.jsx` per surface. That is not "inspiration." It is a contract: exact hexes,
exact type ramp, exact motion, exact component anatomy. The job of a 1:1 port is not
to reinterpret it but to *reproduce* it on a different substrate, and then to *prove*
the reproduction.

The trap is treating fidelity as a guess — "this looks about right." On a foreign
substrate it never is. The only thing that worked was a tight loop: render a state,
crop the panel, look at the pixels, compare to the prototype, fix one token, repeat.
Fidelity is a loop, and the loop needs an instrument (below). Skip the instrument and
you ship "approximately the design," which on a compliance product reads as
carelessness.

## Translate the design system first, not the screens

The first file was not a screen. It was `theme.slint` — a single `Theme` global that
mirrors `tokens.css` one-for-one: brand colors, the warm-paper/evergreen surfaces, the
risk-tier system (the product's most-repeated signal: color + keyframe-shape +
approval-presentation), the sealed-ledger palette, the type families, spacing, radii,
motion durations. Then `typography.slint`, `icons.slint`, `primitives.slint`. Only
then screens.

This ordering is the whole game. When every component reads `Theme.risk-high` instead
of `#b07b00`, a wrong color is a one-line fix in one place — exactly the web kit's
`var(--*)` discipline. When you skip the token layer and inline hexes, you get a
hundred independent bugs and no single source of truth. The kit *is* the port; the
screens are assembly.

## The selectable-text fix: the requirement hiding in plain sight

The sharpest piece of feedback was operational, not aesthetic: *"text is highlightable
in the prototype; in our other Slint apps it is not."* A plain Slint `Text` cannot be
selected. The answer is one component: `SelectableText`, a read-only `TextInput`.
`read-only: true` blocks editing but still allows mouse selection and clipboard copy.
Style it to look like a label, and content text — hashes, DIDs, outputs, code — is
suddenly copyable everywhere.

The lesson generalizes: the most valuable fix in a port is often the one the prototype
gets for free (the browser selects text) and the native substrate does not. Hunt for
those. They are invisible when present and infuriating when absent, and they are the
difference between "a demo" and "a tool."

## The Slint semantics that bite (a field guide)

None of these are hard once known. All of them cost time when learned by collision.
Write them down; that is the point of this essay.

1. **Elements default to `width: 100%; height: 100%` of their parent.** A component
   that `inherits Rectangle` does *not* size to its inner content. Give containers
   explicit sizes or bind to a child's `preferred-height`. The blank-window bug was
   mostly this: a `Chrome` with no height collapsed to zero.

2. **A conditional `if` is not a stretchable layout slot.** `if cond:
   HorizontalLayout { vertical-stretch: 1 }` as a direct child of a layout does not
   receive the stretch. Wrap the conditional in an always-present
   `Rectangle { vertical-stretch: 1 }` and let the `if` fill it.

3. **Several intuitive names are reserved.** `color` is a type, `row`/`col` are grid
   properties, `clip` is the clipping bool. Don't name an input property `color`,
   `row`, or `clip`. Use `tint`, `slot`, `item`.

4. **A component root cannot reference `parent`.** Indent/size from the call site (a
   wrapper layout), not from the root binding.

5. **`@children` cannot live under a conditional.** To collapse content, keep
   `@children` unconditional inside a `Rectangle { clip: true; height: open ?
   body.preferred-height : 0px }`.

6. **Width-derived layout creates binding loops.** Reading `root.width` to decide
   child inclusion feeds back into layout info. Drive responsive breakpoints
   *imperatively* in a `changed width => { … }` handler that writes plain bools — no
   media queries, no loop. Likewise, never size a zone off a *derived* layout height
   (`cs.height * 0.4`); use a non-layout `Rectangle` whose height is driven top-down,
   and position children off `self.height`.

7. **Path icons port almost verbatim.** Slint `Path { commands: "M…" }` takes
   SVG path data on a viewbox. The whole Lucide-style set became one `Path` per icon
   driven by a `name` ternary; circles/rects convert to arc/line subpaths. Stroke
   width scales with size to hold the optical ratio.

8. **Fonts embed at compile time.** `import "font.ttf";` at the top of a `.slint`
   registers a family by name — cleaner than runtime registration (which moved behind
   an unstable feature in 1.16).

## The instrument: headless software rendering

The fidelity loop needs to capture frames, and a terminal-launched GUI in a headless
shell has no window surface — so `take_snapshot()` (which re-renders from the
presented surface) returns transparent. The fix is to render *without* a window:
`MinimalSoftwareWindow` + a tiny custom `Platform`, drawing into a
`PremultipliedRgbaColor` buffer, un-premultiplied to RGBA8 for a PNG. No GPU, no event
loop, no surface. That, plus cropping panels at 2× and viewing the *real* PNG (not a
hand-rolled sampler — mine ignored PNG row filters and lied), is the entire fidelity
methodology. Build it before the screens; it pays for itself in the first hour.

## The discipline that keeps a port honest: the front end never decides

Citrate Studio is a compliance product. The temptation in a UI port is to let a
`.slint` file compute the answer — to make the "Approve" button decide approval. The
discipline is the opposite: the UI is a **viewport and an intent submitter.** Quorum,
separation-of-duties (`CO ⊥ SO`, Auditor read-only, no self-approval), the payload
hash-pin — all computed in Rust and handed to the UI as ready-to-render state
(`approval-roster`, `quorum-met`). The Slint never short-circuits a gate.

This is not pedantry. It is what makes the next step a *swap* instead of a *rewrite*:
today the core is modeled in-process; tomorrow it is the real `citrate-agent-core`
`cdylib`; the UI does not change because it never knew the difference. And it is what
keeps the audit honest — there is exactly one place where policy is decided, and it is
not the front end.

## What transfers to gui-native

This sprint was framed as the *forward-looking UI kit for every Citrate native app*.
Concretely, what carries over: `theme.slint` (the token layer), `typography.slint`
(`SelectableText` + styles), `icons.slint`, `primitives.slint`, and the headless
snapshot harness. The field guide above is the other deliverable — the next port
should not rediscover that a conditional doesn't stretch. Extract the kit files into a
shared crate (`citrate_ui_kit`, the way `gui-native` already vendors its Slint
primitives) and `gui-native` inherits a tested, token-true, selectable-text foundation
instead of re-deriving one.

## The honest assessment

I am fast at well-specified reproduction: given a precise prototype, the breadth of
nine surfaces was not the hard part. I am slow when a *tool* fails rather than the
*work* — I burned an hour conflating a headless-capture problem with a layout bug, and
I trusted my own broken verifier over the actual artifact. The next person should know
that the UI breadth is cheap and the substrate's semantics are the cost, and that the
single highest-leverage early investment is a trustworthy way to *see what you built*.
Build the instrument; translate the tokens; then the screens are assembly.
