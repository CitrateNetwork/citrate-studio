---
created: 2026-06-04T06:45:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-9
---

# Technical Journal — June 4, 2026 (packaging)

## Wiring a gate so it activates itself

### The problem with "TODO: sign the build"

A release pipeline that can't sign — because the certs live in someone's keychain, not the
CI environment — has two bad options and one good one. The bad ones: omit signing entirely
(then it's forgotten, and a contributor with certs has to reinvent it), or make signing
*required* (then the pipeline fails for everyone without certs, including the open build).
The good one: write the signing step *fully*, and gate it on the presence of its secret, so
it skips cleanly when the secret is absent and runs untouched when it's present.

```yaml
- name: macOS sign + notarize
  if: runner.os == 'macOS' && env.APPLE_CERT_P12 != ''
  env:
    APPLE_CERT_P12: ${{ secrets.APPLE_CERT_P12 }}
    ...
  run: |
    echo "$APPLE_CERT_P12" | base64 --decode > cert.p12
    security import cert.p12 -k build.keychain -P "$APPLE_CERT_PASSWORD" ...
    codesign --deep --force --options runtime --timestamp --sign "Developer ID Application: ..." "$APP"
    xcrun notarytool submit "$APP" ... --wait
    xcrun stapler staple "$APP"
```

The `if: env.APPLE_CERT_P12 != ''` is the whole trick. Without the secret, the step is a
no-op and the job still produces an *unsigned* `.app`. With the secret configured, the same
step signs and notarizes — no code change, no "uncomment this." The gate is present,
dormant, and self-activating.

### The bundle metadata is just data

`cargo-bundle` reads `[package.metadata.bundle]` from `Cargo.toml` — `cargo build` ignores
it entirely, so adding it can't break the build (confirmed: 0 issues after). The icon array
points at PNGs; on macOS, `cargo bundle` assembles them into the `.icns`. The icons came
straight from the brand SVG:

```sh
for sz in 32 64 128 256 512 1024; do
  rsvg-convert -w $sz -h $sz assets/brand/citrate_mark_green.svg -o assets/icon/icon_${sz}.png
done
```

One source of truth (the SVG), six rasterizations, no hand-drawn icon to drift.

### The deliverable was a document

The unusual thing about this sprint: its most valuable artifact is `COMPLETION_STATUS.md`,
not code. When a project's headline finish-line items are gated on credentials, people, and
sibling repos you don't control, the honest contribution from inside one repo is to *make
the state legible* — a table that marks each capability ✅ real / 🟡 modeled / ⏳ remaining /
🔒 gated, and names what unlocks each gate. That's not documentation as an afterthought;
it's the engineering output, because the alternative — a README that says "complete" over a
modeled playback — is a bug in the most expensive place to have one.

### Lesson

Write the steps you can't run yet, and gate them on their prerequisite so they skip cleanly
and activate themselves. Keep packaging metadata as data so it can't break the build. And
when the finish line depends on things outside the repo, ship the precise map of what's real
and what gates the rest — the map is the work.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*STUDIO-9 · gated signing · SVG→icon set · COMPLETION_STATUS.md as the deliverable.*
