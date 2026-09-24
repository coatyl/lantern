/**
 * Command palette: open via Ctrl+K, filter, run a command, Escape restores
 * focus.  Covers the library-home first-run hint and the workspace path.
 */

import { test, expect } from "@playwright/test";
import { useFixture } from "./fixtures/use-fixture";

test("library home points at Ctrl+K and the palette opens Settings", async ({
  page,
}) => {
  await page.goto("/");

  await expect(
    page.getByText("Press Ctrl+K (⌘K) to open the command palette."),
  ).toBeVisible();

  await page.keyboard.press("Control+K");
  const palette = page.getByRole("dialog", { name: "Command palette" });
  await expect(palette).toBeVisible();

  await page.keyboard.type("settings");
  await page.keyboard.press("Enter");

  await expect(palette).not.toBeVisible();
  await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();
});

test.describe("with fixture workspace", () => {
  test.beforeEach(async ({ page }) => {
    await useFixture(page);
  });

  test("Ctrl+K opens the palette; Escape restores focus", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("tabpanel")).toBeVisible();

    const settingsBtn = page.getByRole("banner").getByRole("button", {
      name: "Settings",
    });
    await settingsBtn.focus();
    await expect(settingsBtn).toBeFocused();

    await page.keyboard.press("Control+K");
    const palette = page.getByRole("dialog", { name: "Command palette" });
    await expect(palette).toBeVisible();
    await expect(palette.getByRole("combobox")).toBeFocused();

    await page.keyboard.press("Escape");
    await expect(palette).not.toBeVisible();
    await expect(settingsBtn).toBeFocused();
  });

  test("Focus search command moves caret into the list search field", async ({
    page,
  }) => {
    await page.goto("/");
    await expect(page.getByRole("tabpanel")).toBeVisible();

    await page.keyboard.press("Control+K");
    const palette = page.getByRole("dialog", { name: "Command palette" });
    await expect(palette).toBeVisible();

    await page.keyboard.type("focus search");
    await page.keyboard.press("Enter");
    await expect(palette).not.toBeVisible();

    await expect(page.locator("[data-lantern-search]")).toBeFocused();
  });

  test("Run pass command opens the proposed-changes preview", async ({
    page,
  }) => {
    await page.goto("/");
    await expect(page.getByRole("tabpanel")).toBeVisible();

    await page.keyboard.press("Control+K");
    const palette = page.getByRole("dialog", { name: "Command palette" });
    await expect(palette).toBeVisible();

    await page.keyboard.type("run pass");
    await page.keyboard.press("Enter");
    await expect(palette).not.toBeVisible();

    await expect(
      page.getByRole("region", { name: "Proposed changes" }),
    ).toBeVisible();
  });
});
