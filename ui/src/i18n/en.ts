/**
 * English translation table: the reference / default locale.
 *
 * Keys are dot-separated and grouped by surface.  Treat this file as the
 * canonical key set: future locales should mirror its keys exactly.
 *
 * v0.0.9 covers the shell (TitleBar / StatusBar / TabBar) and the
 * SettingsModal rail labels.  Deeper component strings are intentionally
 * left for follow-up milestones; the framework + shell coverage is
 * enough to prove the pattern.
 */

import type { Translations } from "./types";

export const en: Translations = {
  // ── Title bar ──────────────────────────────────────────────────────────
  "titleBar.open":             "Open",
  "titleBar.library":          "Library",
  "titleBar.libraryHint":      "Library (Ctrl+Shift+L)",
  "titleBar.tools":            "Tools",
  "titleBar.tools.diff":       "Compare tabs…",
  "titleBar.tools.deadLinks":  "Check dead links…",
  "titleBar.tools.merge":      "Merge documents…",
  "titleBar.settings":         "Settings",
  "titleBar.minimize":         "Minimize",
  "titleBar.maximize":         "Maximize",
  "titleBar.close":            "Close",

  // ── Status bar ─────────────────────────────────────────────────────────
  "statusBar.offline":         "Offline",
  "statusBar.bookmarks":       "{n} bookmarks",
  "statusBar.folders":         "{n} folders",
  "statusBar.separators":      "{n} separators",
  "statusBar.modified":        "modified",

  // ── Tab bar ────────────────────────────────────────────────────────────
  "tabBar.untitled":           "Untitled",
  "tabBar.closeTab":           "Close tab",
  "tabBar.context.closeTab":           "Close tab",
  "tabBar.context.closeOthers":        "Close other tabs",
  "tabBar.context.closeToRight":       "Close tabs to the right",
  "tabBar.context.closeAll":           "Close all tabs",

  // ── Inline rename ──────────────────────────────────────────────────────
  "rename.placeholder":        "New name…",

  // ── Settings (pane labels) ─────────────────────────────────────────────
  "settings.title":            "Settings",
  "settings.close":            "Close settings",
  "settings.pane.general":     "General",
  "settings.pane.keyboard":    "Keyboard",
  "settings.pane.logs":        "Logs",
  "settings.pane.about":       "About",

  // ── Common ─────────────────────────────────────────────────────────────
  "common.cancel":             "Cancel",
  "common.save":               "Save",
  "common.saving":             "Saving…",
  "common.saved":              "Saved ✓",

  // ── Toasts (v0.0.11 QoL slice 1) ────────────────────────────────────────
  "toast.dismiss":             "Dismiss",

  // ── Empty states (v0.0.11 QoL slice 1; warm-archive voice) ──────────────
  "empty.tree":                "No folders in this archive yet.",
  "empty.tree.description":    "Open a bookmark export that already has a folder tree, or add a folder to begin.",
  "empty.list":                "Nothing filed here yet.",
  "empty.list.description":    "Add a bookmark, or move one in from another folder.",
  "empty.search":              "Nothing in this archive matches '{query}'.",
  "empty.search.description":  "Try a shorter query, or ease the filters.",
  "empty.deadlinks.preRun":    "Ready to check this archive.",
  "empty.deadlinks.allGreen":  "All {count} links in this archive responded.",
  "empty.logs":                "No log entries yet.",
  "empty.logs.description":    "When Lantern records activity, it will appear here.",

  // ── Loading shared (v0.0.11 QoL slice 1) ────────────────────────────────
  "loading.generic":           "Loading…",

  // ── Command palette ─────────────────────────────────────────────────────
  "commandPalette.title":        "Command palette",
  "commandPalette.placeholder":  "Type a command…",
  "commandPalette.empty":        "No matching commands.",
  "commandPalette.hint":         "↑↓ to move · Enter to run · Esc to close",
  "commandPalette.shortcut":     "Ctrl+K / ⌘K",
  "commandPalette.openFile":     "Open file",
  "commandPalette.export":       "Export…",
  "commandPalette.runPass":      "Run pass",
  "commandPalette.search":       "Focus search",
  "commandPalette.toggleTheme":  "Toggle theme",

  // ── Library home (the stacks; the workspace is inside a volume) ─────────
  "library.region":                    "Library",
  "library.title":                     "Library",
  "library.subtitle":                  "A private collection of bookmark files on this machine.",
  "library.empty.title":               "Your library is empty",
  "library.empty.description":         "Open a bookmark file to start a private collection. Files you open stay listed here.",
  "library.open":                      "Open file…",
  "library.recentHeading":             "Recent volumes",
  "library.recentCount":               "{n} volumes",
  "library.clearRecent":               "Clear",
  "library.clearing":                  "Clearing…",
  "library.mostRecent":                "Most recent",
  "library.openVolume":                "Open {name}",
  "library.recovery.title":            "Recover previous session",
  "library.recovery.description":      "Lantern did not shut down cleanly last time. Reopen the previous files?",
  "library.recovery.dismiss":          "Dismiss",
  "library.recovery.restore":          "Restore",
  "library.recovery.restoring":        "Restoring…",
  "library.recovery.failed":           "Could not restore the previous session.",
  "library.recovery.partial":          "Restored {restored} files; {failed} could not be reopened.",
  "library.recovery.more":             "+{n} more",
  "library.paletteHint":               "Press Ctrl+K (⌘K) to open the command palette.",
  "library.hint":                      "Ctrl+O to open · Ctrl+S to save · Ctrl+, settings · Chrome, Firefox, Edge, Safari exports supported",
};
