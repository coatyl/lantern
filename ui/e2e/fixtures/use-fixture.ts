/**
 * Shared helper that wires the browser-stubs fixture flag before the React
 * tree mounts.
 *
 * Specs call `useFixture(page)` in their `test.beforeEach` hook; the page
 * loads with `localStorage.lantern.test.fixture === "small"` so the
 * `tauri-core.ts` stub serves the synthetic document defined in
 * `ui/src/browser-stubs/fixtures/small.ts`.
 */

import type { Page } from "@playwright/test";

export type FixtureName = "small";

export async function useFixture(page: Page, name: FixtureName = "small") {
  await page.addInitScript((value) => {
    try {
      localStorage.setItem("lantern.test.fixture", value);
    } catch {
      // Some browsers throw when localStorage is disabled; ignore so the
      // suite at least opens the welcome screen.
    }
  }, name);
}
