/**
 * E2E coverage of US-001 (open file → three-pane layout) and US-002
 * (browse the folder tree, list pane updates).
 *
 * The browser-stubs fixture pre-opens a synthetic document so the welcome
 * screen is bypassed and the three-pane workspace mounts immediately.
 * This is the minimal happy path: confirm the tree shows the seeded
 * folders and clicking one populates the list pane.
 */

import { test, expect } from "@playwright/test";
import { useFixture } from "./fixtures/use-fixture";

test.beforeEach(async ({ page }) => {
  await useFixture(page);
});

test("US-001: workspace mounts with the fixture document", async ({ page }) => {
  await page.goto("/");

  // The three-pane layout uses role="tabpanel" on the main region.
  await expect(page.getByRole("tabpanel")).toBeVisible();

  // Status bar reports a non-zero bookmark count from the fixture.
  await expect(page.getByText(/bookmarks/i)).toBeVisible();
});

test("US-002: tree pane lists folders and clicking one updates the list", async ({ page }) => {
  await page.goto("/");

  // Folder names from the fixture.
  await expect(page.getByText("News",      { exact: true })).toBeVisible();
  await expect(page.getByText("Reference", { exact: true })).toBeVisible();
  await expect(page.getByText("Tools",     { exact: true })).toBeVisible();

  // Clicking the "News" entry in the tree should reveal its bookmarks.
  await page.getByText("News", { exact: true }).first().click();
  await expect(page.getByText("Hacker News")).toBeVisible();
});
