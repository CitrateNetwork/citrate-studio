#!/usr/bin/env python3
"""Build an HTML gallery from the visual-harness captures.

Usage: gallery.py <out_dir> "<name|env>" ...
"""
import sys
import os
import html

out = sys.argv[1]
specs = sys.argv[2:]

cards = []
for spec in specs:
    name, _, env = spec.partition("|")
    png = f"{name}.png"
    if not os.path.exists(os.path.join(out, png)):
        continue
    label = name.split("-", 1)[1].replace("-", " ") if "-" in name else name
    env_disp = html.escape(env.strip()) or "default state"
    cards.append(
        f"""
    <figure class="card">
      <a href="{png}" target="_blank"><img src="{png}" loading="lazy" alt="{html.escape(label)}"></a>
      <figcaption>
        <span class="name">{html.escape(label)}</span>
        <code>{env_disp}</code>
      </figcaption>
    </figure>"""
    )

page = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<title>Citrate Studio — visual e2e gallery</title>
<style>
  :root {{ color-scheme: light dark; }}
  body {{ font: 14px/1.5 system-ui, sans-serif; margin: 0; background: #0f1a13; color: #cde7d6; }}
  header {{ padding: 22px 28px; border-bottom: 1px solid #ffffff14; }}
  h1 {{ margin: 0; font-size: 18px; letter-spacing: .3px; }}
  header p {{ margin: 6px 0 0; color: #8aa595; font-size: 12.5px; max-width: 70ch; }}
  .grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(360px, 1fr)); gap: 18px; padding: 24px 28px; }}
  .card {{ margin: 0; background: #16261c; border: 1px solid #ffffff12; border-radius: 10px; overflow: hidden; }}
  .card img {{ width: 100%; display: block; background: #efeadd; }}
  figcaption {{ padding: 9px 12px; display: flex; justify-content: space-between; align-items: baseline; gap: 10px; }}
  .name {{ font-weight: 600; text-transform: capitalize; }}
  code {{ color: #8ecc09; font-size: 11px; word-break: break-all; }}
</style></head>
<body>
  <header>
    <h1>Citrate Studio — visual e2e gallery</h1>
    <p>{len(cards)} surfaces, rendered headlessly via the Slint software renderer (this is a
    native app — not Playwright). Each tile is reached through the app's real env seeds, shown
    in green. Click to open full-size.</p>
  </header>
  <div class="grid">{''.join(cards)}
  </div>
</body></html>
"""

with open(os.path.join(out, "index.html"), "w") as f:
    f.write(page)
print(f"  ✓ gallery: {len(cards)} surfaces")
