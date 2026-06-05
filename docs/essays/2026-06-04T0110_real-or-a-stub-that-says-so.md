---
created: 2026-06-04T01:10:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-4
---

# Real, or a Stub That Says So

**In a tool that handles security, there are exactly two honest states for any
primitive: it is real, or it is a stub that announces itself. The dangerous third state
— a fake that behaves like the real thing — is the one that demos beautifully and fails
catastrophically. STUDIO-4 is a study in refusing the third state. Written so every
future surface in this app, and the next one, inherits the rule.**

## The thesis: a fake security primitive is worse than a missing one

A missing feature is a known gap. A *faked* security feature is a hidden lie, and the
lie is load-bearing: people make decisions — to approve, to deploy, to trust — on the
belief that the green checkmark means what it says. When the dock showed
`"3c77…b412"` as a signer fingerprint, that string was a fake. It looked exactly like a
real `signer_id`. Nobody verifying the build by eye could tell that pressing "Sign"
appended a tuple to a vector and computed nothing. The UI was *performing* security.

For a compliance product, performed security is the worst possible failure mode,
because the entire value proposition is that the compliance is real. STUDIO-4's job was
to convert every security primitive in the approval path from performance to fact — or,
where it can't yet be fact, to a stub that says so out loud.

## Real means computed one way and reused everywhere

The tell of fake security is divergence: the fingerprint in the dock, the id in
Settings, and the identity in the core are three different strings that merely *look*
related. The tell of real security is a single source: `signer_id = SHA-256(pubkey)`,
computed once, identical in the dock, the roster panel, and the runtime's
`SignerRoster`. When STUDIO-4 made the keys real, the three strings collapsed into one
value — `6d5e…4628` is Reviewer everywhere — and that collapse *is* the proof. You
cannot get one identity in three places by faking it; you get it by computing it once.

The same principle governs the signature itself. "Sign" now produces an actual ed25519
signature over the capsule's payload hash, and the code **verifies that signature
before it counts**. Not because verification of your own fresh signature can plausibly
fail, but because the habit of trusting only what you've verified is the habit that
keeps the rest of the system honest. A signature you didn't check is indistinguishable,
in your data structures, from one you faked.

## A stub that errors is a different species from a stub that lies

You cannot make everything real at once. PIV/CAC smartcards and FIDO2 authenticators
need hardware this environment doesn't have. The question is what to do at that
boundary, and there are two stubs available:

- The lying stub: pretend to sign, return a plausible signature, let the gate pass. It
  demos flawlessly. It is a trapdoor.
- The honest stub: `return Err(SignError::SurfaceNotWired("PIV"))`. The button does
  nothing but report that the surface isn't wired. It demos as *incomplete* — which is
  the truth.

STUDIO-4 took the second, and pinned it with a test asserting the PIV surface *errors*
rather than signs. This is the single most important decision in the sprint, and it is
almost invisible: a one-line `Err`. But it is the line that separates a security tool
you can trust the absences of from one you can't. When a reviewer sees PIV greyed out
with "not yet wired," they know exactly where the real work stops. When they see PIV
sign successfully on a machine with no smartcard, they've been lied to and don't know
it.

The seam has a name and a type precisely so that the absence is legible. `SigningSurface`
isn't a boolean "supported?"; it's an enum where two variants sign and two return a
typed error. The type system carries the honesty.

## Fail-closed is honesty made visible

The final move is to make the security truth something the user *watches happen*. The
planset's rule — no roster ⇒ High/Critical cannot be approved — could have lived as an
assertion buried in a function. Instead, disenroll a role in Settings and its dock row
goes disabled, reading "No signer enrolled (fail-closed)," and the gate cannot proceed.
The fail-closed posture isn't described; it's demonstrated, on screen, on demand,
backed by a test. A security property you can trigger and observe is one a skeptic can
*falsify* — and falsifiability is what makes a claim more than marketing.

## The cost, paid honestly

Refusing the third state has a price, and STUDIO-4 paid it visibly. To keep the demo
alive, first run seeds real keys on the **FileBacked** surface — a private key in a
plaintext file. That's a real risk, named in the code, the retro, and here, with the
mitigation stated (Keyring is one click away and stores the secret in the OS keyring)
and the fix scheduled (STUDIO-5 defaults new enrollments to Keyring). The honest move
isn't to have no debt; it's to have no *hidden* debt. A risk written down with a plan is
managed; a risk performed away is a landmine.

## The rule, generalized

Every primitive this app touches — a signature, an audit hash, a policy decision, an
anchor on a chain — faces the same fork. Make it real (computed one way, verified,
reused), or stub it so it announces itself (a typed error, a greyed control, a labelled
absence). Never the third thing. The discipline scales down to a single `Err` variant
and up to the whole product's credibility, and it is the same discipline at both ends:
*in a tool people trust to tell them the truth about risk, the code must never tell them
a comfortable lie.*

---

*Essay by Claude (AI assistant), reflecting on the STUDIO-4 sprint with Larry.*
