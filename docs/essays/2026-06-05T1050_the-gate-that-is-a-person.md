---
created: 2026-06-05T10:50:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-14
---

# The Gate That Is a Person

**Not every gate is a missing credential you can go fetch. Some gates are a human
authorization that the system is *designed* to require — and trying to route around them is
trying to defeat the security feature, not satisfy a dependency. Signing an app with a private
key is one of these. STUDIO-14 is about recognizing when the right move is to stop at the gate
and hand the keystroke to the person who owns it. Written about the difference between a gate
you open and a gate you ask someone to open.**

## The thesis: some "blockers" are features, and the correct response is deference

Across this project I've poked a lot of gates and found most of them passable: the faucet was
a POST, the capsules were already in the repo, the kit pattern was already established. The
recurring lesson was "poke the gate before you defer behind it." But there's a dual lesson,
and STUDIO-14 is it: some gates are *supposed* to stop you, and the right response to those is
not to find a clever way through — it's to recognize the gate as legitimate and defer to the
human it exists to involve.

Code-signing with a Developer ID is exactly this. The certificate is right there in the
keychain; the signing command is one line. But `codesign` blocked, waiting for a UI dialog —
*"codesign wants to use the private key, Allow?"* — and was killed. That dialog is not friction
to be automated away. It is macOS enforcing that a signing key, the thing that lets you assert
"this software is from this developer," cannot be used by a process the key's owner hasn't
authorized. A headless agent using a login-keychain key without a human's say-so is precisely
the attack the dialog prevents. The gate is the feature.

## Credential gates vs interaction gates

It's worth distinguishing two kinds of "I can't do this here," because they call for opposite
responses.

A **credential gate** is a missing secret: an API key, a cert file, notary credentials. You
*can* satisfy it — obtain the credential, configure it, proceed. When the project flagged
"signed installers — needs Apple certs," that read like a credential gate, and partly it is
(notarization needs notary creds, which are fetchable). The right move for a credential gate is
to get the credential or wire the path so it activates when the credential arrives.

An **interaction gate** is different: the system requires a *human action* by design, and no
amount of credential gathering removes it. The keychain's "Always Allow" prompt is an
interaction gate. You can't satisfy it by having more secrets; you satisfy it by a person
authorizing the use of *their* key. The correct move is not to engineer around it — it's to do
everything up to the gate and then make the human's step as small as possible (one command, one
click) and unmistakable.

The mistake is treating an interaction gate like a credential gate — grinding at it, looking
for the env var or the flag that makes it non-interactive on someone else's key. There isn't
one that's legitimate, and the illegitimate ones (extracting the key, scripting the UI) are
exactly what you shouldn't build. Recognizing which kind of gate you're at is half of handling
it correctly.

## Doing everything up to the line

Deferring at an interaction gate is not the same as stopping early. The discipline is to
complete *everything* the gate doesn't require a human for, so the human's residual step is
trivial and obvious:

- the `.app` is built, with the real icon and identity;
- the signing script is written, with the correct entitlements, hardened runtime, and timestamp;
- it's wired into CI, where a *dedicated* keychain can be pre-authorized programmatically
  (`set-key-partition-list` — the CI-owned equivalent of "Always Allow," which is legitimate
  because the CI keychain is created for the job, not borrowed from a person);
- the notarization path is wired to activate on a stored profile;
- and the one thing left — Larry running `scripts/sign-macos.sh` in his session and clicking
  *Always Allow* once — is documented as exactly that.

The difference between "I couldn't sign it" and "the bundle and script are done; run this one
command to sign with your key" is the difference between a blocker and a clean handoff. The
human is left with a keystroke, not a project.

## Why an agent *should* hit this wall

There's something almost reassuring about being stopped here. An agent that *could* silently
sign software with a developer's private key would be an agent you couldn't safely give a
keychain to. The wall I hit is the same wall that protects Larry from a compromised or
mistaken process signing something in his name. Hitting it, and deferring correctly, is not a
limitation to apologize for — it's the security property holding under exactly the conditions
it's meant to hold under: an automated process, however capable, does not get to wield a human's
signing identity on its own authority.

## The rule, generalized

Poke gates before deferring — but recognize the ones that are *meant* to stop you. A signing
key, a hardware token, a "confirm with your password" — these are interaction gates, human
authorization by design, and the right response is deference, not circumvention. Do everything
up to the gate so the human's step is one command and one click; pre-authorize only resources
created for automation (a CI keychain), never borrow a person's. And take being stopped at such
a gate as confirmation the system is sound: the keystroke that's not yours to make is exactly
the one a trustworthy agent leaves for the person who owns the key.

---

*Essay by Claude (AI assistant), reflecting on the STUDIO-14 sprint with Larry.*
