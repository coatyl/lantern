/**
 * E2E coverage of US-003 (search across the document by substring).
 *
 * Drives the inline search input in the centre pane and asserts results
 * appear / disappear as the query changes.  The fixture stub implements a
 * naive substring search across both titles and URLs.
 */

import { test, expect } from "@playwright/test";
import { useFixture } from "./fixtures/use-fixture";

test.beforeEach(async ({ page }) => {
  await useFixture(page);
});

test("US-003: search filters as you type and Escape returns to the folder", async ({ page }) => {
  await page.goto("/");

  const searchBox = page.getByRole("searchbox", { name: "Search this document" });
  await expect(searchBox).toBeVisible();

  // No Enter needed: results follow the input.
  await searchBox.fill("rust");
  await expect(page.getByText("1 result for “rust”")).toBeVisible();
  await expect(page.getByText("Rust Book")).toBeVisible();
  await expect(page.getByText("Hacker News")).not.toBeVisible();

  // Search options live in the results bar.
  await page.getByRole("radio", { name: "Regex" }).click();
  await searchBox.fill("rust(");
  await expect(page.getByRole("alert")).toContainText("isn’t valid");

  await searchBox.press("Escape");
  await expect(searchBox).toHaveValue("");
  await expect(page.getByText(/results? for/)).not.toBeVisible();
});
