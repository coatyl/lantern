/**
 * E2E coverage of US-008 (preview proposed changes) and US-009 (apply the
 * approved subset).
 *
 * Clicks "Run pass" in the detail pane to spawn the inline PreviewPanel
 * dialog, asserts its `aria-modal` shell renders, then clicks Apply.
 * The fixture stub returns a single synthetic change; once applied the
 * detail pane shows a "Applied N change(s)" summary.
 */

import { test, expect } from "@playwright/test";
import { useFixture } from "./fixtures/use-fixture";

test.beforeEach(async ({ page }) => {
  await useFixture(page);
});

test("US-008/US-009: run pass shows preview and Apply produces a summary", async ({ page }) => {
  await page.goto("/");

  // Detail pane has a "Run pass" button (sanitization section).
  const runPass = page.getByRole("button", { name: "Run pass" });
  await expect(runPass).toBeVisible();
  await runPass.click();

  // PreviewPanel mounts as role="dialog" with aria-label "Proposed changes".
  const dialog = page.getByRole("dialog", { name: "Proposed changes" });
  await expect(dialog).toBeVisible();

  // Apply the (single) proposed change.
  const applyBtn = dialog.getByRole("button", { name: /^Apply\b/ });
  await expect(applyBtn).toBeEnabled();
  await applyBtn.click();

  // After apply the dialog closes and a "Applied …" summary is shown.
  await expect(dialog).not.toBeVisible();
  await expect(page.getByText(/Applied \d+ change/)).toBeVisible();
});
