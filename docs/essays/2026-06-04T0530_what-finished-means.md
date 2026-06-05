---
created: 2026-06-04T05:30:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-7
---

# What "Finished" Means

**"Finished" is not a feeling. It's a property you can check: every line is either used and
correct, or honestly annotated as to why it isn't. A codebase that looks done but is shot
through with `allow(dead_code)`, hardcoded values that contradict the real data, and
unwraps nobody has reasoned about is not finished — it's *abandoned at a plausible-looking
point*. STUDIO-7 is about the difference, and the discipline that closes it. Written so the
bar for "done" is the same the next time we say it.**

## The thesis: every suppression is a deferred decision

`#[allow(dead_code)]` is not a neutral annotation. It's a note that says "I'm choosing not
to decide what to do about this yet." That's fine as a temporary marker — but a temporary
marker that never gets revisited becomes a permanent lie. The five module-level allows in
this codebase had each started as "I'll wire this soon"; left alone, they would have masked
not just the intended item but every future piece of dead code in those modules, forever.

So the first act of finishing is to *make the deferred decisions*. Turn off every
suppression and confront what it was hiding. The compiler names it precisely; you can't
argue with "never used." Then each finding forces a real choice — and there are exactly
three honest ones.

## The three honest fates of unused code

**Delete it.** Most of what surfaced was STUDIO-2 auth scaffold that STUDIO-4's `signing.rs`
had superseded: a `signer_id_from_pubkey`, a `RosterEntry`, an `enroll`. These weren't
unfinished — they were *replaced*. Keeping a draft alongside the final is not thoroughness;
it's clutter that future readers must mentally route around. Deleting it makes the codebase
smaller and more honest in the same stroke. The hardest thing about deleting code is the
sunk-cost feeling that writing it should mean keeping it. It shouldn't. The design moved on;
the code should too.

**Annotate it precisely.** Some code is unused in *one* build and essential in another. The
default-build signing path (`sign_for`) is dead in the `core-live` binary — because there
the dock signs through the real `ApprovalQueue` — but alive in the default binary and in
tests. A blanket `allow` would call it dead everywhere, which is false. The precise
annotation, `#[cfg_attr(feature = "core-live", allow(dead_code))]`, says exactly what's
true: allowed-dead under core-live, warned-on under default. The annotation carries
information instead of erasing it. That's the line between a justification and a suppression
— a justification tells you *why and when*; a suppression just says *stop bothering me*.

**Wire it.** The most interesting category: code that's correct, proven by tests, and simply
not reached from the UI. The STUDIO-6 dispatch and doctor bridges sat exactly here. The
shortcut was obvious — `allow(dead_code)`, move on. But that freezes a real capability at
"works in a test, invisible to the user," which is its own kind of debt: a feature you built
and then hid. The honest move was to *connect* it — a button that dispatches a real capsule
through wasmtime, a Health Report fed by the real `DoctorReport` with a summary computed from
the actual results rather than a hardcoded string. The dead code became the feature it was
always meant to be. "Wire it, don't allow it" is the rule that keeps a hardening pass from
degrading into a deletion spree.

## Hardening is not "delete what the linter flags"

There's a failure mode of thoroughness worth naming, because it's seductive: treating every
lint as a mandate to remove. A clean `signature_count` accessor that mirrors the runtime's
own API got flagged as unused-in-binary. The lazy "thorough" move is to delete it. But that
would be *minimizing the architecture for a reason that isn't important* — sacrificing a real,
coherent API to satisfy a tool. The right move was to keep it and annotate the narrow truth
(`#[cfg_attr(not(test), allow(dead_code))]`: used by tests, allowed-dead otherwise). The
discriminator is always the same question: does this line *do real work* somewhere, or is it
residue? Residue goes. Real work stays, correctly annotated. A linter can find candidates; it
can't make that judgment for you.

## The numbers that look scary and aren't

"54 unwraps" is the kind of metric that triggers a rewrite reflex. The triage showed 47 were
test assertions — where a panic *is* the failure signal — and the 7 real ones were each
provably safe: a `last()` immediately after a `push`, single-thread mutex locks that can't be
poisoned in their usage, a guarded env var, entropy and runtime construction where failure is
catastrophic and unrecoverable anyway. The work wasn't to replace 54 unwraps; it was to
*reason about* them and write down why each is safe. Finishing is often less about changing
code than about being able to defend the code that's there. A number you've understood is not
the same as a number you've reduced.

## The honest residue

Finishing does not mean zero seams. It means every seam is *labelled and gated* rather than
hidden. Chain writes need a funded key — gated on `CITRATE_ANCHOR_KEY`, surfaced in the UI as
"read-only (no anchor key)." Signed-capsule dispatch waits on the upstream packer
(CIT-AGENT-3e) — so today's real execution rides a loudly-logged dev opt-in. The L0 agent loop
is CIT-AGENT-3, upstream. None of these is studio's code to write; all of them are stated in
the code and the docs. That's the difference between a deferral and a debt: a deferral is
written down with its reason and its gate; a debt is a silence you'll trip over later.

## The rule, generalized

Before you call anything finished, run the audit that can't be fooled: turn off the
suppressions and read what the compiler says. Then give every unused thing one of three honest
fates — delete the superseded, annotate the cfg-conditional, wire the merely-unreached — and
never the fourth, silent one. Reason about every unwrap until you can defend it or replace it.
Label and gate every seam you can't close. "Finished" is when there's nothing left that you're
choosing not to look at.

---

*Essay by Claude (AI assistant), reflecting on the STUDIO-7 sprint with Larry.*
