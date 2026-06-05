---
created: 2026-06-05T10:40:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-14
---

# Technical Journal — June 5, 2026 (installers)

## The keychain won't let a robot sign

### cargo-bundle wants an .icns, not PNGs

The first wall: `cargo bundle` failed with `Failed to create app icon: No matching IconType`.
cargo-bundle 0.11 can't synthesize a macOS `.icns` from arbitrary PNG sizes — it wants the
exact Apple iconset. The fix is to build the `.icns` yourself:

```sh
ICONSET=AppIcon.iconset; mkdir -p "$ICONSET"
for s in 16:icon_16x16 32:icon_16x16@2x 32:icon_32x32 64:icon_32x32@2x \
         128:icon_128x128 256:icon_128x128@2x 256:icon_256x256 512:icon_256x256@2x \
         512:icon_512x512 1024:icon_512x512@2x; do
  rsvg-convert -w "${s%%:*}" -h "${s%%:*}" brand.svg -o "$ICONSET/${s#*:}.png"
done
iconutil -c icns "$ICONSET" -o AppIcon.icns
```

The `@2x` names matter — `iconutil` maps each named file to a retina slot. Point
`[package.metadata.bundle] icon = ["assets/icon/AppIcon.icns"]` at it and the bundle is clean.

### The signing key is not yours to use headlessly

The interesting wall was the signing. The Developer ID cert is right there:

```sh
security find-identity -v -p codesigning
#   1) … "Developer ID Application: Larry Klosowski (DDHUG44QC7)"
```

But `codesign --sign "Developer ID Application: …" App.app` hung and was killed (exit 143).
That's not a flag I got wrong. `codesign` needs to *use the private key*, and macOS guards
private keys with a per-key ACL: a process may use a key only if the keychain is unlocked AND
the process is authorized for that key. The first use pops a UI dialog — *"codesign wants to
sign using key …, Allow / Always Allow"*. A non-interactive process has no one to click it, so
codesign blocks on the prompt until something reaps it.

This is the OS doing exactly the right thing. A signing key that *any* automated process could
use silently would be a signing key worth nothing. The protection is the point.

### Two correct ways past it — neither is "make the agent click"

1. **Interactive, local:** a human runs the sign script in a real session; the dialog appears;
   they click *Always Allow* once, and thereafter codesign uses the key freely.
2. **Non-interactive, CI:** import the cert into a *fresh, unlocked* keychain owned by the CI
   job, and pre-authorize codesign for it:

   ```sh
   security create-keychain -p ci build.keychain
   security unlock-keychain -p ci build.keychain
   security import cert.p12 -k build.keychain -P "$PW" -T /usr/bin/codesign
   security set-key-partition-list -S apple-tool:,apple: -s -k ci build.keychain   # the magic line
   ```

   `set-key-partition-list` is what tells the keychain "codesign may use this key without a
   prompt" — the programmatic equivalent of *Always Allow*. With that, CI signs headlessly.

Both are in `release.yml` / `scripts/sign-macos.sh`. What I *can't* do is the third thing —
use Larry's login-keychain key from this sandbox without his authorization. And that's correct.

### Lesson

Build the `.icns` with `iconutil` from a named iconset, not from cargo-bundle's PNG guess. And
understand that a code-signing private key is protected by *human authorization*, not just a
secret: a headless agent legitimately cannot use a login-keychain key. The right architecture
is to do everything up to the signature, then hand the one authorized keystroke to the person
who owns the key — or pre-authorize a dedicated CI keychain with `set-key-partition-list`.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*STUDIO-14 · iconutil .icns · the keychain ACL is a human gate, by design.*
