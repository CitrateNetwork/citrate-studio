# Visual e2e harness

Confirms **every** Citrate Studio surface renders correctly, as images — and gates against
visual regressions.

## Why this is a snapshot harness, not Playwright

Citrate Studio is a **native Rust + Slint** desktop app. Its UI is drawn by the Slint
**software renderer** to a pixel buffer — there is **no DOM, no browser, no web page**. So
browser-automation tools (Playwright, Cypress, Selenium) have nothing to attach to and
**cannot drive this app**. The native equivalent — and what lives here — is a
**software-renderer snapshot harness**: drive every state through the app's real env seeds,
render each headlessly to a PNG, build a gallery, and diff against committed golden images.

Playwright *is* the right tool for **web** surfaces — the original `Citrate Studio.html`
prototype and any future cdylib-backed web re-skin. A scaffold for that is in
[`../../tests/web-playwright/`](../../tests/web-playwright/), pointed at where it applies.

## Run it

```sh
scripts/visual-harness.sh capture   # render all surfaces → docs/visual/gallery/index.html
scripts/visual-harness.sh update    # render all → refresh docs/visual/baseline/ (golden images)
scripts/visual-harness.sh check     # render all → diff vs baselines (exit 1 on drift) — CI gate

STUDIO_CORE_LIVE=1 scripts/visual-harness.sh capture   # real dispatch/doctor/chain states
```

- **`gallery/index.html`** — a browsable, labelled grid of every surface (open in a browser).
- **`baseline/`** — committed golden PNGs; `check` flags any surface whose mean pixel diff
  exceeds the threshold (writes a `*_diff.png` heat image for the offender).

## Coverage (19 surfaces)

idle · running · the High gate dock · a completed run (real `wasmtime` output badges under
`core-live`) · signed-in (+ KYC badge) · the L2 Inspector · the L4 Code Drawer · the Health
Report · Break-Glass · the agent chat · Settings × 6 (RBAC, roster, grants, runtime, policy
with the live chain row, account) · the tampered Audit Scrubber · onboarding · the Beginner
workspace.

Each is reached through the same env seeds the app honors (`CITRATE_STUDIO_SEED`, `OVERLAY`,
`SECTION`, `SIGNEDIN`, `TAMPER`, `VIEW`, `WS`, …), so the harness drives real app states, not
mocks.
