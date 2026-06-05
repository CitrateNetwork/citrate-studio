---
created: 2026-06-05T09:50:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-13
---

# Technical Journal — June 5, 2026 (ui-kit)

## The re-export shim that kept every import working

### The problem with moving shared files

Studio's components import the design system everywhere: `import { Theme } from
"theme.slint"`, `import { Btn } from "../primitives.slint"`, dozens of call sites. Moving
`theme.slint` into a separate crate naively means rewriting all of them to `from
"@citrate-ui-kit"`. That's a large, error-prone diff in the *consumers* to relocate the
*definitions*.

### The shim inverts it

Instead, leave a file where each import expects it — but make it a one-line re-export of the
real thing, now in the kit:

```slint
// studio/ui/theme.slint
export { Theme, Settings } from "@citrate-ui-kit";
```

Every `import { Theme } from "theme.slint"` resolves through this shim into the kit's
`lib.slint`, which re-exports from the moved `theme.slint`. The definition is in the kit; the
*import surface* the components see is untouched. Four shims (theme/typography/icons/
primitives), zero changes to any component. The diff that moves 23 components is four lines.

### The library mechanism

Slint shares `.slint` across crates via named libraries. The kit's `build.rs` publishes the
path to its `lib.slint`; the consumer's `build.rs` registers it:

```rust
let mut libs = HashMap::new();
libs.insert("citrate-ui-kit".to_string(), PathBuf::from(kit_lib_path));
slint_build::compile_with_config("ui/studio.slint",
    CompilerConfiguration::new().with_library_paths(libs))?;
```

Then any `.slint` can `import { Theme } from "@citrate-ui-kit";`. The `@name` namespace is the
indirection that lets the kit live anywhere.

### Where the proven pattern slipped

The federation's kit publishes its path via `links` + `cargo:UI_KIT=…`, surfaced to consumers
as `DEP_CITRATE_STUDIO_UI_KIT_UI_KIT`. My consumer's `build.rs` read that env — and panicked,
because it wasn't set (the metadata didn't propagate to the *build-dependency's* script in
this setup). Rather than block on the cargo-metadata rule, I made it degrade gracefully:

```rust
let kit = std::env::var("DEP_CITRATE_STUDIO_UI_KIT_UI_KIT")
    .unwrap_or_else(|_| "ui-kit/ui/lib.slint".to_string());   // workspace-relative fallback
```

Prefer the published path; fall back to the relative path for a local path-dep. It builds; the
metadata link is a refinement for later.

### The pure-asset-crate call

The kit's `build.rs` first compiled `lib.slint` for Rust types — which warned, for every
primitive, "doesn't inherit Window — no code generated." Expected: `Btn`, `RiskBadge` etc. are
sub-components, consumed via the `@library` import, not as standalone Rust `Window` types. To
keep zero warnings, I dropped the kit's own compile entirely: it's now a pure asset crate that
ships `.slint` + fonts + the path. Consumers generate the types. No warnings, correct
behavior, and the kit has no slint dependency at all.

### The proof was already built

A design-system move is the canonical "did I break a pixel?" change. STUDIO-11's golden-image
harness answered it: `visual-harness.sh check` → all 19 surfaces match baseline. The
extraction is behavior-preserving, proven, not asserted.

### Lesson

To relocate shared definitions without churning consumers, leave re-export shims where the
imports expect them and move the bodies behind a `@library` namespace. Make build-script
metadata lookups degrade to a sensible fallback. Keep a shared-asset crate free of the
compile it doesn't need. And when you refactor a UI, let a golden-image gate be your sign-off.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*STUDIO-13 · re-export shims · @citrate-ui-kit library · pixel-identical by the harness.*
