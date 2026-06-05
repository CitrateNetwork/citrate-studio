---
created: 2026-06-04T02:20:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-5
---

# Technical Journal — June 4, 2026 (later)

## A flat TOML you can trust, and a probe that tells the truth

### Persisting config without pulling serde

The setup config is six flat keys: workspace, tenant, runtime, oversight,
capsules, first_prompt, onboarding_complete. The "obvious" path is
`serde`-derive + the `toml` crate — two dependencies and a derive macro to
persist a struct with no nesting. For a flat key/value file I wrote the
reader/writer by hand instead:

```rust
// write
format!("workspace = \"{}\"\noversight = \"{}\"\nonboarding_complete = {}\n", …)
// read
for line in s.lines() {
    let Some((k, v)) = line.split_once('=') else { continue };
    match k.trim() { "workspace" => c.workspace = unquote(v.trim()), … }
}
```

That's the whole format: `key = "value"` for strings, bare `true`/`123` for
bools/ints, `#` comments skipped. No dependency, fully under our control.

### The bug a hand-rolled parser hands you

The first reader used `v.trim_matches('"')` to strip the surrounding quotes.
That's wrong the moment a value *contains* a quote. The first prompt
`Reconcile last night's "ledger"` serializes (with escaping) to:

```
first_prompt = "Reconcile last night's \"ledger\""
```

`trim_matches('"')` strips **every** leading and trailing `"` — including the
escaped ones — and mangles the value back to `…ledger\`. The fix is to strip
*exactly one* delimiter quote per side and let the un-escaper handle the rest:

```rust
v.strip_prefix('"').and_then(|x| x.strip_suffix('"')).unwrap_or(v)
```

The test `toml_roundtrips` (deliberately containing an embedded quote) caught it
on the first run. That's the trade with a hand-rolled format: you save the
dependency and you inherit the parser's edge cases. Write the test that has the
nasty character in it.

### A probe that says what it found

Onboarding's runtime step used to assert "I found a few on your network" no
matter what. Now it actually looks:

```rust
pub fn probe(endpoint: &str, path: &str) -> bool {
    ureq::get(&format!("{endpoint}{path}"))
        .timeout(Duration::from_millis(400))
        .call()
        .is_ok()
}
```

It probes Ollama (`:11434/api/tags`) and llama.cpp (`:8080/health`), and the
onboarding line is built from the result: name the runtimes that answered, or —
the usual case on a fresh box — "no local server answered, so I'll bind the
embedded Gemma." On localhost a refused connection returns immediately, so two
probes don't stall the step; the 400ms timeout only bites if something is half-up.
The test stands a `tiny_http` server on an ephemeral port (probe → true) and
hits `127.0.0.1:1` (probe → false).

### Fail-closed install, with real signatures

`install_capsules` verifies each capsule's ed25519 publisher signature and keeps
only what checks out:

```rust
for c in sources {
    if verify_capsule(c) { ok.push(c.name) } else { bad.push(c.name) }
}
```

The demo set is five capsules signed by one generated publisher key, plus a
`rogue.exfiltrate` with a zero signature. The test asserts 5 install and the
rogue is rejected — fail-closed, with the same `signing::verify` the dock uses.

### The injection that paid a debt

`new_state` did disk I/O (loading the roster, now also the config), which meant
any test building a `RunState` touched `$HOME`. Split it: `new_state_with(roster)`
takes the roster and defaults the config — pure, for tests — and the production
`new_state` does the two disk reads on top. Now no unit test reads the user's
home directory.

### Lesson

A config file is only as trustworthy as its parser and its honesty. Hand-roll the
format if it's flat, but write the test with the awkward character in it. Make
discovery *report*, not *assert*. And keep I/O out of constructors so your tests
describe the code, not the machine.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*STUDIO-5 · citrate-studio.toml · real runtime probe · fail-closed capsule install.*
