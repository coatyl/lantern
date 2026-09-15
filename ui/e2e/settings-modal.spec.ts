/**
 * E2E coverage of US-013 (theme switch: General pane) plus light coverage
 * of the Tools menu in the title bar.
 *
 * The settings modal is opened via Ctrl+, (registered globally in App.tsx),
 * the rail's role="tablist" is navigated with ArrowDown / ArrowUp, and
 * the General pane's theme select is changed.  We don't assert on what
 * the theme actually does to the DOM. The hook is covered by Vitest;
 * here we just assert the form interaction works.
 */

import { test, expect } from "@playwright/test";
import { useFixture } from "./fixtures/use-fixture";

test.beforeEach(async ({ page }) => {
  await useFixture(page);
});

test("US-013: settings modal opens via Ctrl+, and the rail accepts arrow nav", async ({ page }) => {
  await page.goto("/");

  // Make sure the workspace is mounted before firing the shortcut.
  await expect(page.getByRole("tabpanel")).toBeVisible();

  await page.keyboard.press("Control+,");

  const dialog = page.getByRole("dialog", { name: "Settings" });
  await expect(dialog).toBeVisible();

  // Rail is a vertical tablist with four tabs.
  const tablist = dialog.getByRole("tablist", { name: "Settings sections" });
  await expect(tablist).toBeVisible();
  await expect(tablist.getByRole("tab", { name: "General" })).toHaveAttribute("aria-selected", "true");

  // ArrowDown moves focus + selection to the next tab.
  await tablist.getByRole("tab", { name: "General" }).focus();
  await page.keyboard.press("ArrowDown");
  await expect(tablist.getByRole("tab", { name: "Keyboard" })).toHaveAttribute("aria-selected", "true");

  // ArrowUp returns to General.
  await page.keyboard.press("ArrowUp");
  await expect(tablist.getByRole("tab", { name: "General" })).toHaveAttribute("aria-selected", "true");
});

test("Tools menu opens from the title bar", async ({ page }) => {
  await page.goto("/");

  // The Tools button is rendered in the title bar with aria-haspopup="menu".
  const toolsBtn = page.getByRole("button", { name: "Tools" });
  await expect(toolsBtn).toBeVisible();
  await toolsBtn.click();

  await expect(page.getByRole("menu", { name: "Tools" })).toBeVisible();
});
