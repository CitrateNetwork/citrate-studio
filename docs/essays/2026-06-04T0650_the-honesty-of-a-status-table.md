---
created: 2026-06-04T06:50:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-9
---

# The Honesty of a Status Table

**The last act of building something is telling the truth about how done it is. That sounds
trivial and is the hardest part, because every incentive pushes toward the comforting label
— "complete," "done," "shipped" — over the precise one. A status table that marks each
capability real / modeled / gated, and names what gates each, is worth more than any
adjective, because it's the one artifact a reader can trust without running the system.
Written so the last thing we build, every time, is an honest account of the rest.**

## The thesis: "done" is a claim, and claims can be audited

When you say a system is complete, you're making a claim that someone will act on — a user
who deploys it, an auditor who attests to it, a teammate who builds on top. If the claim is
loose ("it works"), the cost of its looseness lands on them, later, at the worst time. A
compliance product makes this acute: the entire value proposition is that its claims about
risk are true. A studio that *claims* it runs the agent end-to-end while the canvas plays a
modeled timeline has, at the finish line, committed exactly the sin it exists to prevent.

So the final engineering act isn't a label. It's an account precise enough to be audited on
paper. `COMPLETION_STATUS.md` maps the four operator capabilities to ✅ real, 🟡 modeled,
⏳ remaining, 🔒 gated — line by line, with what gates each open item. A reader learns the
real state of the system without trusting a single adjective. That's the difference between
documentation and an attestation: an attestation is structured so it can be *checked*.

## Why the comforting label is the dangerous one

Nine sprints of real work create a strong pull toward declaring victory. The auth is real,
the roster is real, the queue is real, the dispatch is real, the chain reads are real, the
doctor is real, the tests are green, the dead code is gone. It would *feel* true to write
"complete." But the planset's own definition of complete requires the canvas to drive a
*live* run — real dispatch, real audit record, real anchor — and that's still the modeled
timeline. The gap between "almost everything is real" and "complete" is small in effort and
total in meaning. Labeling across it isn't optimism; it's a false statement that someone
downstream will discover the expensive way.

The discipline is to let the strength of the work stand on its own — *hardened release
candidate* is a strong, true claim — and refuse to round it up. Precision is not modesty.
It's the same fail-closed instinct the product is built on, applied to its own description.

## Gated is not the same as unfinished

A subtler honesty: distinguishing what's *unfinished* from what's *gated*. They look alike
in a checklist — both are unchecked boxes — but they're different in kind. Unfinished work is
yours to do. Gated work is blocked on something outside the repo: a code-signing cert, an
external auditor, a sibling shell that doesn't exist yet, a funded key. Conflating them
either makes you look negligent (treating a missing cert as your incomplete work) or makes
you hide a real dependency (treating your unfinished work as "gated, not my problem").

The status table separates them explicitly. Signed installers: 🔒 gated on certs, with the
CI step already written and dormant. The canvas live-run: ⏳ remaining, a functional piece
with a UX decision attached. Naming the *kind* of each gap is as important as naming the gap,
because it tells the reader who acts next — you, an admin, an auditor, an upstream team.

## Wire the gate; don't omit the step

There's a builder's corollary to all this. When a step is gated, the temptation is to leave
it out and write a TODO. The better move is to write the step *fully* and gate it on its
prerequisite, so it's present, dormant, and self-activating. The release workflow signs and
notarizes the macOS build — guarded by `env.APPLE_CERT_P12 != ''`. No cert, no signing, but
an unsigned bundle still ships; add the cert, and the same workflow signs with no code
change. The gate is documented *as executable code*, which is a stronger form of honesty
than a sentence in a README — it can't drift from what actually happens, because it *is*
what happens.

## The map is the work

The unusual conclusion of a packaging sprint, when the headline items are gated on the
outside world, is that the most valuable thing you produce is a document. That's not a
consolation prize. When a system's true state is hard to see — modeled here, real there,
gated elsewhere — making it *legible* is real engineering, because the alternative is a team
operating on a wrong belief about what they have. A status table that an auditor can read and
trust is, at the finish line, worth as much as a feature. Maybe more, because a feature you
can't accurately describe is a liability wearing a checkmark.

## The rule, generalized

Finish by telling the truth about how finished you are. Build a status account precise enough
to audit on paper: real / modeled / remaining / gated, line by line, with what gates each.
Refuse to round "almost complete" up to "complete." Distinguish what's yours to finish from
what's blocked on the world, and name who acts next. Wire your gates as dormant, self-
activating code rather than omitting them. The last artifact you ship should be the one that
lets someone trust all the others — not because you said so, but because they can check.

---

*Essay by Claude (AI assistant), reflecting on the STUDIO-9 sprint with Larry — and the nine-sprint arc.*
