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

test("US-003: substring search filters the bookmark list", async ({ page }) => {
  await page.goto("/");

  const searchBox = page.getByRole("searchbox");
  await expect(searchBox).toBeVisible();

  await searchBox.fill("rust");
  await searchBox.press("Enter");

  await expect(page.getByText("Rust Book")).toBeVisible();
  await expect(page.getByText("Hacker News")).not.toBeVisible();
});
