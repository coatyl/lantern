/**
 * E2E coverage of US-001 (open file → three-pane layout) and US-002
 * (browse the folder tree, list pane updates).
 *
 * The browser-stubs fixture pre-opens a synthetic document so the library
 * home is bypassed and the three-pane workspace mounts immediately.
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

  // Status bar (role="contentinfo" footer) reports a bookmark count from the
  // fixture. Scoped to the footer so it doesn't collide with the tree's
  // "Bookmarks" root entry.
  await expect(page.getByRole("contentinfo").getByText(/bookmarks/i)).toBeVisible();
});

test("US-002: tree pane lists folders and clicking one updates the list", async ({ page }) => {
  await page.goto("/");

  // Folder names appear as buttons in the tree pane. Scope to the tabpanel
  // so "Tools" doesn't collide with the title-bar Tools menu.
  const workspace = page.getByRole("tabpanel");
  await expect(workspace.getByRole("button", { name: "News" })).toBeVisible();
  await expect(workspace.getByRole("button", { name: "Reference" })).toBeVisible();
  await expect(workspace.getByRole("button", { name: "Tools" })).toBeVisible();

  // Clicking the "News" entry in the tree should reveal its bookmarks.
  await workspace.getByRole("button", { name: "News" }).click();
  await expect(page.getByText("Hacker News")).toBeVisible();
});
