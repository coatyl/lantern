/**
 * E2E coverage of US-008 (preview proposed changes) and US-009 (apply the
 * approved subset).
 *
 * Clicks "Run pass" in the detail pane, which swaps the list and detail
 * panes for the review surface.  The fixture proposes a URL fix, a title
 * fix, a folder rename and a duplicate deletion; the deletion starts
 * unselected.  Applying closes the review and raises an "Applied N
 * changes" toast.
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

  const review = page.getByRole("region", { name: "Proposed changes" });
  await expect(review).toBeVisible();
  await expect(review.getByText("Lantern repo")).toBeVisible();
  await expect(review.getByText("GitHub (saved twice)")).toBeVisible();

  // Destructive changes start unselected: 3 of 4.
  await expect(review.getByText("3 of 4 selected")).toBeVisible();
  const applyBtn = review.getByRole("button", { name: "Apply 3 changes" });
  await expect(applyBtn).toBeEnabled();
  await applyBtn.click();

  await expect(review).not.toBeVisible();
  await expect(page.getByText(/Applied \d+ change/)).toBeVisible();
  // The workspace is back.
  await expect(page.getByRole("button", { name: "Run pass" })).toBeVisible();
});

test("review: Escape discards without applying", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Run pass" }).click();
  const review = page.getByRole("region", { name: "Proposed changes" });
  await expect(review).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(review).not.toBeVisible();
  await expect(page.getByText(/Applied \d+ change/)).not.toBeVisible();
});
