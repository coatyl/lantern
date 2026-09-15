/**
 * Theme application: resolves the user's `ThemeSetting` to an effective
 * dark/light mode and writes `data-theme` on <html>.
 *
 *   • "dark"   → always dark (no media-query listener)
 *   • "light"  → always light (no media-query listener)
 *   • "system" → follows `prefers-color-scheme: light` and updates live
 *                whenever the OS preference flips
 *
 * The dark theme is the CSS default (`:root` block in index.css), so we only
 * set the attribute when light is requested.  This keeps the SSR/initial
 * render path simple: pages load dark by default and only flip if needed.
 */

import { useEffect } from "react";
import { ipc } from "../ipc";
import type { ThemeSetting } from "../ipc/types";

// ---------------------------------------------------------------------------
// Helpers: exported so the test/setup files can poke at them
// ---------------------------------------------------------------------------

/** Apply the resolved theme to <html>.  "dark" removes the attribute, "light"
 *  sets it. */
export function applyTheme(resolved: "dark" | "light"): void {
  const root = document.documentElement;
  if (resolved === "light") {
    root.setAttribute("data-theme", "light");
  } else {
    root.removeAttribute("data-theme");
  }
}

/** Resolve a user setting to either "dark" or "light" using the current OS
 *  preference if necessary.  Defaults to "dark" when `matchMedia` is missing
 *  (e.g. older test harnesses). */
export function resolveTheme(setting: ThemeSetting): "dark" | "light" {
  if (setting === "dark")  return "dark";
  if (setting === "light") return "light";
  // setting === "system"
  if (typeof window === "undefined" || !window.matchMedia) return "dark";
  return window.matchMedia("(prefers-color-scheme: light)").matches
    ? "light"
    : "dark";
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

/**
 * Mount-time hook: load the persisted theme setting, apply it, and listen for
 * OS preference changes whenever the user is on "system".
 *
 * The dependency on `bumpKey` lets a parent (e.g. App, after the settings
 * modal closes) request a re-read from disk without remounting the whole
 * tree.  Pass any value that changes, typically `settingsModalOpen`.
 */
export function useTheme(bumpKey?: unknown): void {
  useEffect(() => {
    let cancelled = false;
    let cleanupMql: (() => void) | null = null;

    (async () => {
      let setting: ThemeSetting = "system";
      try {
        const s = await ipc.getSettings();
        setting = s.theme;
      } catch {
        // Browser-mode / pre-Tauri context: fall through to system default.
      }
      if (cancelled) return;

      // Apply once.
      applyTheme(resolveTheme(setting));

      // For "system", listen for OS preference flips and re-apply.
      if (setting === "system" && typeof window !== "undefined" && window.matchMedia) {
        const mql = window.matchMedia("(prefers-color-scheme: light)");
        const handler = (e: MediaQueryListEvent) => {
          applyTheme(e.matches ? "light" : "dark");
        };
        // Tauri ships modern WebView2/WebKit, so we can rely on the
        // standard EventTarget API.  `addListener` is the legacy fallback
        // for ancient Safari; not required here.
        mql.addEventListener("change", handler);
        cleanupMql = () => mql.removeEventListener("change", handler);
      }
    })();

    return () => {
      cancelled = true;
      if (cleanupMql) cleanupMql();
    };
  }, [bumpKey]);
}
