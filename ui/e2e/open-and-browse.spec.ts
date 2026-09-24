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

  // The status bar (role="contentinfo") reports the fixture's bookmark count.
  await expect(page.getByRole("contentinfo").getByText(/bookmarks/i)).toBeVisible();
});

test("US-002: the folder tree is keyboard navigable and drives the list", async ({ page }) => {
  await page.goto("/");

  const tree = page.getByRole("tree", { name: "Folders" });
  for (const name of ["News", "Reference", "Tools"]) {
    await expect(tree.getByRole("treeitem", { name })).toBeVisible();
  }

  await tree.getByRole("treeitem", { name: "News" }).click();
  await expect(page.getByText("Hacker News")).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Breadcrumb" })).toContainText("News");

  // Arrow down to the next folder and open it with Enter.
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(page.getByText("MDN Web Docs")).toBeVisible();
});
