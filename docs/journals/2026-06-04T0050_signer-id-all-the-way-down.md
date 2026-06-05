---
created: 2026-06-04T00:50:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-4
---

# Technical Journal — June 4, 2026 (late)

## signer_id all the way down

### The goal

Make the approval dock's fingerprints *real*. Before STUDIO-4 they were string
literals — `"3c77…b412"` — pretty and meaningless. After, every enrolled role
carries a generated ed25519 keypair, and the fingerprint you see is
`SHA-256(pubkey)` of that key: the **same** `signer_id` the dock shows, the
Settings roster shows, and the runtime's `SignerRoster` records. One identity,
computed one way, in three places.

### The mechanics

ed25519-dalek 2.x makes the core small:

```rust
let secret = random_secret();                 // 32 bytes from getrandom
let sk = SigningKey::from_bytes(&secret);
let pubkey = sk.verifying_key().to_bytes();    // 32 bytes
let signer_id = hex(Sha256::digest(&pubkey));  // 64 hex chars — matches the runtime
```

Signing is `sk.sign(msg).to_bytes()` → `[u8; 64]`; verifying is
`VerifyingKey::from_bytes(pk)?.verify(msg, &sig)`. The one rule I held: **the
dock verifies the signature it just produced before it counts it.** `sign_for`
signs, then re-verifies against the stored pubkey, and only then does the
signature exist. A signature you didn't verify is a rumor.

### Surfaces: real, or honestly absent

The signing *surface* (where the private key lives) is a tier, and the tier is
truthful:

- **FileBacked** — secret in the local JSON (dev). Signs.
- **Keyring** — secret in the OS keyring, loaded on demand. Signs.
- **PIV/CAC, FIDO2** — `return Err(SignError::SurfaceNotWired(..))`.

That last line is the whole point of the seam: the hardware path is *typed and
absent*, not faked. A test (`piv_surface_is_stubbed_not_faked`) pins that it
errors. The difference between a stub that errors and a stub that returns a fake
signature is the difference between honest and dangerous.

### Fail-closed you can watch

`approval_roster` iterates the gate's *required roles*, not the roster — so a
role with no enrolled signer becomes a disabled row reading "No signer enrolled
(fail-closed)" instead of silently vanishing. Disenroll Reviewer in Settings and
the High gate visibly cannot be satisfied. The planset's security truth ("no
roster ⇒ High/Critical can't be approved") stopped being a sentence and became
something the UI *does*, backed by a test.

### What bit (and what it taught)

`new_state()` calls `load_or_seed()`, which reads the platform config dir. That
means any test building a `RunState` touches `$HOME` — a real `roster.json` gets
read or written under the user's config path. The crypto and roster tests are
clean (they use `seed_demo()` + temp files), but the dock test had to
**explicitly override `s.roster = seed_demo()`** to be deterministic, because
otherwise it inherits whatever the last real run persisted.

That's a testability smell with a clear name: *a constructor that does I/O*.
`new_state` should take an injected roster (or a config path), so nothing reads
the user's home directory in a unit test. Logged as a STUDIO-5 debt rather than
patched in place, because the fix is a small refactor of the state constructor
that deserves its own diff.

### Lesson

Real crypto is cheap to do correctly and the correctness is mostly discipline:
compute the identity one way and reuse it everywhere; verify what you sign before
you trust it; make absent surfaces *error*, not lie; and keep I/O out of your
constructors so your tests don't depend on the machine they run on.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*STUDIO-4 · ed25519 enrollment · signer_id = SHA-256(pubkey), one identity everywhere.*
