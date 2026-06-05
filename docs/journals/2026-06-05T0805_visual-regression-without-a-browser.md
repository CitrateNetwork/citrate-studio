---
created: 2026-06-05T08:05:00Z
branch: main
author: Claude Opus 4.8 (1M context)
status: active
sprint: STUDIO-11
---

# Technical Journal — June 5, 2026 (visual harness)

## Visual regression for a native app, without a browser

### The app is already drivable — from the outside

The harness needed to reach 19 distinct states. It didn't need any new app code, because
every prior sprint had added an env seed for its own headless screenshots:

```
CITRATE_STUDIO_SEED=gate          # the High gate dock
CITRATE_STUDIO_OVERLAY=health     # the Doctor report
CITRATE_STUDIO_SECTION=policy     # settings → policy (live chain row)
CITRATE_STUDIO_TAMPER=1           # the broken audit chain
CITRATE_STUDIO_SIGNEDIN=0x… KYC=verified
CITRATE_STUDIO_SHOT=out.png       # render headlessly to PNG and exit
```

So the harness is a table of `name|ENV` and a loop:

```bash
for spec in "${SPECS[@]}"; do
  name="${spec%%|*}"; vars="${spec#*|}"
  env $vars CITRATE_STUDIO_SHOT="$OUT/$name.png" "$BIN" >/dev/null 2>&1
done
```

The lesson in retrospect: a headless render entrypoint + env-addressable state *is* a test
API. Build it for screenshots, get an e2e harness for free.

### The diff is a mean over pixels

`scripts/visual-diff.py` is deliberately simple — `ImageChops.difference`, mean over all
channels, threshold:

```python
diff = ImageChops.difference(a, b)
mean = sum(sum(p) for p in diff.getdata()) / (len(diff.getdata()) * 3)
if mean > 2.0:   # 0–255; AA jitter stays well under this
    diff.save(png.replace(".png", "_diff.png"))   # heat image of the change
    regressed.append(png)
```

A mean of 2/255 tolerates sub-pixel anti-aliasing while catching any real visual change. The
`*_diff.png` heat image is the important affordance: when CI flags a regression, you *see*
exactly which pixels moved, not just a number.

### Test the test

A regression gate is worthless if you've never watched it fail. So I corrupted it on purpose:

```sh
cp baseline/01-studio-idle.png baseline/04-run-done.png   # wrong baseline
python3 scripts/visual-diff.py gallery baseline
#   ✗ REGRESSED 04-run-done.png: mean diff 5.74 > 2.0
cp /tmp/orig04.png baseline/04-run-done.png               # restore
```

5.74 > 2.0 — it fires. Now I trust the green.

### The platform gotcha

The catch with software-renderer golden images: anti-aliasing isn't bit-identical across
OSes. macOS-rendered baselines would noise-fail on a Linux runner. The pragmatic fix is to
pin the CI gate to the baseline's platform (macOS here) and document per-platform
re-baselining. The "proper" fix is a perceptual metric (SSIM) or per-OS baseline sets — more
machinery than a 2/255 mean threshold warrants for a single-platform gate today.

### Why not Playwright

Worth stating in the code, not just the head: Playwright automates a browser via the
DevTools/WebDriver protocol. A native Slint app exposes no such surface — it draws pixels to
a buffer. There is literally nothing for Playwright to connect to. The native analogue of
"navigate, act, screenshot, assert" is "seed, render, diff" — which is this harness. The
README says so plainly so no one wastes a day trying to point Playwright at the binary.

### Lesson

Build a headless render entrypoint and address state by env, and you've built a visual e2e
API. Keep the diff simple but *see* the changed pixels. Watch the gate fail once. Pin golden
images to one platform or go perceptual. And match the tool to the substrate — a browser
automator can't drive a pixel buffer.

---

*Journal entry by Claude (AI assistant), for Larry (human architect).*
*STUDIO-11 · 19 env-seeded captures · mean-pixel golden gate · validated by deliberate drift.*
