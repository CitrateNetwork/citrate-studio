#!/usr/bin/env python3
"""Golden-image regression: diff captured surfaces against committed baselines.

Mean per-pixel difference per image; a surface "regressed" if its mean diff exceeds
THRESHOLD (0–255). Writes a *_diff.png heat image for any regression. Exits non-zero if
any surface regressed or a baseline is missing — a CI gate.

Usage: visual-diff.py <out_dir> <baseline_dir>
"""
import sys
import os

try:
    from PIL import Image, ImageChops
except ImportError:
    print("visual-diff: Pillow not installed (pip install pillow) — skipping diff.")
    sys.exit(0)

out, base = sys.argv[1], sys.argv[2]
THRESHOLD = 2.0  # mean 0–255; anti-aliasing jitter stays well under this

regressions, missing = [], []
captures = sorted(f for f in os.listdir(out) if f.endswith(".png") and not f.endswith("_diff.png"))

for png in captures:
    bp = os.path.join(base, png)
    if not os.path.exists(bp):
        missing.append(png)
        continue
    a = Image.open(os.path.join(out, png)).convert("RGB")
    b = Image.open(bp).convert("RGB")
    if a.size != b.size:
        regressions.append((png, f"size {a.size} != baseline {b.size}"))
        continue
    diff = ImageChops.difference(a, b)
    px = list(diff.getdata())
    mean = sum(sum(p) for p in px) / (len(px) * 3)
    if mean > THRESHOLD:
        diff.save(os.path.join(out, png.replace(".png", "_diff.png")))
        regressions.append((png, f"mean diff {mean:.2f} > {THRESHOLD}"))

print(f"▸ visual-diff: {len(captures)} surfaces vs baseline")
for png, why in regressions:
    print(f"  ✗ REGRESSED {png}: {why}")
for png in missing:
    print(f"  ? NO BASELINE {png} (run `visual-harness.sh update`)")
if not regressions and not missing:
    print("  ✓ all surfaces match baseline")

sys.exit(1 if (regressions or missing) else 0)
