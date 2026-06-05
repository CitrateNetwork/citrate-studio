# Playwright — for the WEB surfaces only

**Playwright drives web browsers. The shipped Citrate Studio is a native Slint app and
cannot be driven by Playwright** (no DOM). Use the native snapshot harness for the desktop
app: [`scripts/visual-harness.sh`](../../scripts/visual-harness.sh).

This directory is the scaffold for Playwright where it *does* apply — a **web** surface:

1. **The original prototype** `Citrate Studio.html` (the React/HTML design the native app was
   ported from), if served locally.
2. **A future web re-skin** backed by the `cdylib` shared core (planset Phase D / STUDIO-9) —
   the same Rust core behind a web front end.

Neither web target exists in this repo yet, so the spec below is a **template**, pointed at a
placeholder `BASE_URL`. When a web surface exists, set `BASE_URL` and these tests drive it the
way the native harness drives the desktop app — by reaching each state and asserting it
visually (`toHaveScreenshot`) plus by behavior.

## Run (once a web target exists)

```sh
cd tests/web-playwright
npm i
BASE_URL=http://localhost:5173 npx playwright test
```
