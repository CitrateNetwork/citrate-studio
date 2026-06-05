---
created: 2026-06-04T22:40:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-2
---

# The Smallest Correct Auth

**Authentication code is mostly trust surface, and trust surface is liability. The
discipline that matters is not adding the most security machinery — it's adding the
least machinery that is still correct, and being able to say *why* each piece earns
its place. Written so the next native client (gui-native) inherits the reasoning, not
just the crate list.**

## The thesis: every dependency in an auth path is a promise you now keep

When you `cargo add` a crate into the code that decides who the user is, you have not
just imported a function — you have adopted a maintainer's threat model, their release
cadence, their transitive tree, and their bugs, into the most sensitive path in your
app. On a Tier-1 audited surface, that is the opposite of free. So the question for
every line of an auth client is not "is this a reasonable library?" but "does removing
it make the system *wrong*?" If the answer is no, the library is liability you chose.

STUDIO-2 started from an ADR that named `reqwest` and `jsonwebtoken` + JWKS. Both are
fine libraries. Both turned out to be removable without making the system wrong. The
sprint's real work was noticing that.

## Reading the spec is cheaper than importing the crate

The ADR's instinct was defensible: an OIDC client validates ID tokens, ID tokens are
JWTs, JWT validation means fetching the issuer's JWKS and verifying the RS256
signature with `jsonwebtoken`. That is the right algorithm for the *implicit* and
*hybrid* flows, where the token arrives at the client through the browser — an
untrusted channel — and the signature is the only thing vouching for it.

But this is a **native code flow**. The client exchanges the authorization code for
tokens by making a direct, client-initiated TLS request to the token endpoint. OIDC
Core §3.1.3.7 is explicit about what that changes: when the ID token is received by
direct communication between the client and the token endpoint over TLS, the client
*may* use the TLS server validation in place of checking the token signature. The TLS
channel already authenticates the issuer. Re-verifying an RS256 signature on top adds
a JWKS fetch, a key cache, a crypto dependency (`ring`, via `jsonwebtoken`), and a
class of failure modes (key rotation, JWKS reachability) — to re-prove something the
transport already proved.

So the JWKS path came out, and in its place: validate `iss`, `aud`, and `exp` — three
string/integer comparisons, no crypto, pure defense-in-depth. The trust surface
shrank by a whole cryptographic dependency, and the system is still correct *by the
spec's own words*. That is the trade the discipline is looking for: less machinery,
same correctness, and a citation you can point an auditor at.

## "Sync-friendly" is a real constraint, not a preference

The second removal was `reqwest` → `ureq`. This looks like taste; it isn't. Slint runs
a single-threaded event loop. `reqwest`'s ergonomic path is async, which means pulling
in a Tokio runtime and then bridging that runtime's futures back into a UI that has no
async executor. `ureq` is blocking and pure-rustls (no OpenSSL system dependency), so
the entire login becomes one blocking function on a worker thread, and the result
crosses back via `invoke_from_event_loop`. The constraint "this app is sync" propagated
correctly into the dependency choice. When you let the runtime model pick the HTTP
client, you avoid importing a second concurrency model you'll spend the rest of the
project fighting.

## Amending your own design is the job, not a failure of it

Here is the part that's easy to get wrong culturally. The ADR was *mine*. Deviating
from it could read as the plan being bad, or as license to wander. The Agentile method
resolves this cleanly: a deviation is allowed, but it must be **written down with its
reason**, in the sprint and the ADR amendment, where the next person and the audit can
see it. That single rule is what makes the honest call the cheap one. Without it, an
agent either asks permission for every micro-decision (slow, and it trains the human to
rubber-stamp) or makes the call silently (fast, and it hides the one thing the audit
needs to see). With it, you change the design in the open and move on.

An ADR should commit hard to the *architecture* and the *constraints* — loopback PKCE
not device flow, OS keyring not plaintext, auth keys never equal signer keys — and hold
its specific crate picks loosely, because the trade-offs only become concrete once the
code exists. STUDIO-2's ADR was right on every architectural axis and wrong on two
libraries. That's a good ADR.

## What "smallest correct" bought

The shipped client is a complete native auth: sign-in, logout, refresh-on-expiry,
keyring storage, a session chip — parity with the web RPs — built on `ureq`,
`tiny_http`, `keyring`, `getrandom`, and no JWKS crypto it doesn't need. The honest
limit is stated plainly: the live end-to-end against a running identity server is
mock-tested here, not CI-proven, and that proof belongs to a federation e2e harness.

The lesson generalizes past auth. The most secure-*looking* system and the most secure
system are not the same; piling on machinery can grow the attack surface faster than it
shrinks the attack. The craft is subtraction with a citation: remove what the
environment already guarantees, keep what it doesn't, and write down which is which.
The next native client should start from this client's dependency list and ask, of each
line, the same question — *does removing this make us wrong?* — because that question,
asked honestly, is the whole of secure design at this layer.

---

*Essay by Claude (AI assistant), reflecting on the STUDIO-2 sprint with Larry.*
