// TEMPLATE — Playwright e2e for a WEB Citrate Studio surface (prototype or re-skin).
// Does NOT run against the native Slint app. Set BASE_URL to a served web build.
//
// Mirrors the native harness's coverage (scripts/visual-harness.sh): reach each surface,
// assert it visually (golden screenshot) AND by behavior. Fill in the selectors when a web
// target exists.
import { test, expect } from "@playwright/test";

test.describe("Citrate Studio (web)", () => {
  test("studio renders", async ({ page }) => {
    await page.goto("/");
    await expect(page).toHaveScreenshot("studio-idle.png");
  });

  test("the High gate requires quorum then resumes", async ({ page }) => {
    await page.goto("/");
    // drive the run to the High gate
    await page.getByRole("button", { name: /play/i }).click();
    await expect(page.getByText(/high/i)).toBeVisible();
    await expect(page).toHaveScreenshot("gate-dock.png");
    // sign two non-conflicting roles → quorum → resume
    await page.getByRole("button", { name: "Sign" }).first().click();
    await page.getByRole("button", { name: "Sign" }).nth(1).click();
    await expect(page.getByText(/quorum met/i)).toBeVisible();
    await page.getByRole("button", { name: /resume/i }).click();
    await expect(page.getByText(/COMPLETE/)).toBeVisible();
    await expect(page).toHaveScreenshot("run-done.png");
  });

  test("the audit scrubber detects tamper", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /simulate tamper/i }).click();
    await expect(page.getByText(/previous_hash break/i)).toBeVisible();
    await expect(page).toHaveScreenshot("scrubber-tampered.png");
  });
});
