---
created: 2026-06-05T09:30:00Z
branch: main
author: Saul Loveman + Claude Opus 4.8 (1M context)
status: archived
sprint: STUDIO-13
---

# STUDIO-13 — Extract the UI kit into a shared crate

**Goal.** Extract the foundational design system (theme, typography, icons, primitives +
fonts) into a reusable `citrate-studio-ui-kit` crate, consumed by Studio **and** a second
shell — closing the planset's "UI kit extracted + consumed by another shell" item.

**Approach (the federation's proven pattern, at Slint 1.16).**
- A `ui-kit/` sub-crate carrying the four foundational `.slint` files + fonts. `build.rs`
  compiles its `lib.slint` and publishes the path via `cargo:UI_KIT=…` (needs `links`), so
  consumers' `build.rs` register it as the `@citrate-ui-kit` Slint library.
- Studio's `ui/{theme,typography,icons,primitives}.slint` become thin **re-export shims**
  (`export { … } from "@citrate-ui-kit"`), so every component import keeps working unchanged.
- A second consumer — `examples/kit-gallery` — renders kit components, proving reuse.
- **The visual harness golden images are the proof**: if the extraction shifts a pixel, the
  STUDIO-11 gate flags it. A clean `visual-harness.sh check` = pixel-identical extraction.

**Why a new kit, not the existing `citrate-ui-kit`:** gui-native's kit is pinned to Slint
1.9; Studio is 1.16 (chosen for the latest features). A version downgrade would regress
Studio, so Studio's kit is the forward-looking 1.16 one (per the planset's framing of Studio
as "the UI kit for all Citrate native apps").

**Plan.**

| # | Step | Status |
|---|---|---|
| 1 | `ui-kit/` crate (Cargo, build.rs, lib.slint, src/lib.rs); move the 4 files + fonts | done |
| 2 | Studio: re-export shims + build.rs library path + workspace + font paths | done |
| 3 | `examples/kit-gallery` second consumer | done |
| 4 | Build green + `visual-harness.sh check` pixel-identical | done |

**Daily updates.**

- 2026-06-05 — kickoff.
- 2026-06-05 — **extracted, pixel-identical.** Moved theme/typography/icons/primitives (23
  components) + 4 fonts into `ui-kit/` (a pure Slint-library asset crate, 1.16). Studio's four
  foundational files are now one-line re-export shims (`export … from "@citrate-ui-kit"`), so
  no component import changed; build.rs registers the kit library (DEP env + relative
  fallback). `examples/kit-gallery` renders kit components via `@citrate-ui-kit` — a second
  consumer. Studio + gallery + core-live build, 35 tests, zero warnings, and
  `visual-harness.sh check` = all 19 surfaces pixel-identical.

**Decisions made.**

- **Re-export shims** keep every consumer import unchanged — the disturbance is contained at
  four lines, not dozens of call sites.
- **Pure asset crate** (no Rust-type compile) avoids the "doesn't inherit Window" warnings for
  primitives; consumers generate their own types via the library import.
- **A new 1.16 kit**, not gui-native's 1.9 `citrate-ui-kit` — a downgrade would regress Studio;
  consolidation is a future federation task.
- **Relative-path fallback** for the library path, since `DEP_…_UI_KIT` didn't propagate to the
  build-dependency's script; the DEP env is preferred when present.

**Exit criteria.**

- [x] Foundational design system lives in `ui-kit/`; Studio consumes it via `@citrate-ui-kit`.
- [x] A second shell (`kit-gallery`) consumes the same kit.
- [x] Default + core-live build green; `visual-harness.sh check` passes (pixel-identical).

**Close note.**

STUDIO-13 extracts the design system into `citrate-studio-ui-kit` — a real, reusable crate
carrying the four foundational `.slint` files + fonts — consumed by Studio (via re-export
shims, zero import churn) and a second `kit-gallery` shell. The extraction is proven
*pixel-identical* by STUDIO-11's golden-image gate (all 19 surfaces match), which is the right
sign-off for a pure refactor. Honest seams: the `DEP_…_UI_KIT` metadata link fell back to a
relative path; the kit is a Slint-asset crate (no Rust types); and the federation now carries
two UI-kit crates (this 1.16 one + gui-native's 1.9) until consolidation. Retro + journal +
essay alongside.
