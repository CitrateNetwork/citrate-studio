---
created: 2026-06-04T02:30:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-5
---

# Setup That Means Something

**An onboarding flow is judged by one question: when the user closes it, is the system
actually configured? Most setup wizards fail this quietly — they collect answers and
discard them, leaving a tour dressed as a setup. STUDIO-5 is about the opposite: making
every step change persistent state, probing the world honestly instead of asserting, and
proving it all by the one behavior that can't be faked — the second launch. Written so the
next first-run experience starts from "what does this step actually do?"**

## The thesis: a setup step that configures nothing is a lie with a progress bar

There is a genre of onboarding that is pure theatre. It shows a checklist, animates each
item to "done," congratulates you — and configures nothing. The checkmarks are about the
*flow*, not the *system*. It's a guided tour wearing a setup's clothes, and the tell is
that uninstalling and reinstalling produces the identical experience, because the first
run never wrote anything down.

The Citrate Studio onboarding *started* in that genre — by necessity. In STUDIO-1 it was a
scripted conversation that picked display strings: "Team workspace," "Ollama :11434," "5
signed capsules." Beautiful, and inert. STUDIO-5's whole job was to convert each of those
strings from a *label* into an *action that persists*. The test of success isn't that the
flow looks good; it's that closing it leaves a `citrate-studio.toml` on disk describing a
real configuration, and that the app behaves differently next time because of it.

## The proof is the second launch

Most properties of a setup flow are easy to fake in a screenshot. Exactly one is not:
what happens the *second* time you open the app. A real setup writes a completion marker
and routes the returning user past onboarding into the product. A fake one shows the
wizard again, because there was nothing to remember.

So the load-bearing line of the whole sprint is small:

```rust
if !config::is_configured() { route_to_onboarding() } else { route_to_studio() }
```

`is_configured()` reads the persisted `onboarding_complete` flag. The flag is written only
when the final step runs `config::save`. That single round-trip — write on completion,
read on launch — is what separates "configured" from "performed." Everything else in the
sprint exists to make that flag *mean* something: that by the time it's set, a runtime was
probed, capsules were verified, an oversight default was chosen, and all of it is on disk.

## Honesty in discovery: report, don't assert

The most tempting place to fake a setup is "discovery," because the script reads so well.
"I scanned your network and found a few model runtimes" is a great line. It is also, on a
fresh machine with nothing installed, a lie — and a security product that lies in its
*setup* has poisoned the well before the user has done anything.

So the runtime step actually probes: an HTTP GET to Ollama's and llama.cpp's local ports,
short timeout, and the onboarding line is *built from the result*. If they answer, it
names them. If nothing answers — the common case — it says exactly that and binds the
embedded model instead. "No local server answered, so I'll use the bundled Gemma" is less
impressive than "I found a few," and it is the version a user can trust, because it
matches what's true on their machine. The discipline generalizes: a setup flow should
*report* the world it found, never *assert* a world that reads well.

## Fail-closed is a setup-time property, not just a runtime one

It's easy to think of "fail-closed" as something the running system does — refuse the
unsigned action, halt on the broken chain. But the *install* step is where the unsafe
thing first gets a chance to enter, and so it's where fail-closed has to start. STUDIO-5's
capsule install verifies each capsule's ed25519 publisher signature and stages only what
checks out; the demo set deliberately includes a `rogue.exfiltrate` capsule with a bad
signature, and it is rejected. The point isn't the demo capsule — it's that the *gate
exists at setup time*, so an unsigned capsule never makes it into the configuration in the
first place. A system that's fail-closed at runtime but trusting at install has merely
moved the trapdoor earlier.

## Pay the debt the next sprint, while it's still small

STUDIO-4 left two debts: a constructor that did disk I/O (so tests touched `$HOME`), and
FileBacked-by-default. STUDIO-5 paid both — `new_state_with` now injects the roster and
config so no unit test reads the user's home directory, and Keyring is the prominent
default. There's nothing clever here; the lesson is about *timing*. A debt named in one
sprint's retro and paid in the next stays a footnote. The same debt carried for five
sprints becomes architecture. Writing the debt down is half the discipline; the other
half is treating "the next sprint" as the deadline, not "eventually."

## The honest edge

Setup configures *intent*; some of that intent's *execution* is still ahead. The chosen
runtime is recorded but not yet bound to a live inference loop; the capsules are verified
but not yet dispatched against real `.cps`; the first prompt is captured but not yet run
by a real agent. Those are STUDIO-6, and saying so plainly is part of the same honesty
that made discovery report instead of assert. A setup that persists a real, verified
*intent* and is clear about which parts execute later is trustworthy. A setup that implies
the whole machine is live the moment you finish is the theatre we started by rejecting.

## The rule, generalized

Every onboarding step faces the same fork the security primitives did two sprints ago:
configure something real and persist it, or perform a checkmark. Never the checkmark
alone. Probe the world and report it; verify before you install; write the completion
marker only when the work is done; and prove the whole thing with the one behavior nobody
can screenshot — the second launch landing the user where their first launch put them. A
setup flow's integrity is not how it looks while you click through it. It's what's true on
disk when you're done.

---

*Essay by Claude (AI assistant), reflecting on the STUDIO-5 sprint with Larry.*
