/**
 * Playwright configuration for Lantern UI E2E specs.
 *
 * v0.1.0 release-readiness slice 1: cross-component flows that are awkward
 * to assert from Vitest (component-tree IPC dance, modal lifecycle,
 * keyboard shortcuts triggering store mutations).  Specs run against the
 * Vite dev server at :5173 with the browser-stubs aliasing IPC; no Tauri
 * runtime is involved.
 *
 * The CI job (`ui-e2e` in `.github/workflows/ci.yml`) is `continue-on-error`
 * for v0.1.0, same posture as the reproducible-build-check.  The
 * framework is in place; flake-tolerance hardening is a follow-up.
 */

import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: "http://localhost:5173",
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        // Lets a machine with a preinstalled Chromium (cloud dev boxes,
        // air-gapped CI) run the suite without `playwright install`.
        launchOptions: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE
          ? { executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE }
          : {},
      },
    },
  ],
  webServer: {
    command: "npm run dev -w lantern-ui",
    url: "http://localhost:5173",
    cwd: "..",
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
