/**
 * Library home: return from a focused volume without losing open tabs,
 * and render the seeded recent-files collection in fixture mode.
 *
 * Existing workspace specs still expect the pre-opened fixture tab to
 * mount the three-pane layout (role=tabpanel) on first paint.
 */

import { test, expect } from "@playwright/test";
import { useFixture } from "./fixtures/use-fixture";

test("library home is empty when no fixture is loaded", async ({ page }) => {
  await page.goto("/");

  await expect(
    page.getByRole("heading", { name: "Your library is empty" }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Open file…" })).toBeVisible();
  await expect(page.getByRole("tabpanel")).toHaveCount(0);
});

test.describe("populated library (fixture)", () => {
  test.beforeEach(async ({ page }) => {
    await useFixture(page);
  });

  test("pre-opened fixture tab still mounts the workspace", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("tabpanel")).toBeVisible();
  });

  test("Library returns home without closing the open tab", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("tabpanel")).toBeVisible();

    await page.getByRole("button", { name: "Library" }).click();

    await expect(page.getByRole("tabpanel")).toHaveCount(0);
    await expect(page.getByRole("region", { name: "Library" })).toBeVisible();
    await expect(page.getByText("Recent volumes")).toBeVisible();
    await expect(page.getByRole("button", { name: "Open small.html" })).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Open bookmarks-firefox.html" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Open chrome-bookmarks.html" }),
    ).toBeVisible();

    // Open tabs survive: the fixture document is still in the tab bar.
    await expect(page.getByRole("tab", { name: /small\.html/ })).toBeVisible();

    await page.getByRole("tab", { name: /small\.html/ }).click();
    await expect(page.getByRole("tabpanel")).toBeVisible();
  });
});
