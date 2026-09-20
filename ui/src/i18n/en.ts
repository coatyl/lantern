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

  // ── Welcome (warm-archive identity) ─────────────────────────────────────
  "welcome.kicker":            "Private archive",
  "welcome.title":             "Lantern",
  "welcome.manifesto":         "A light in a dark room. The archive stays on this machine.",
  "welcome.open":              "Open file…",
  "welcome.hint":              "Ctrl+O to open · Ctrl+S to save · Ctrl+, settings · Chrome, Firefox, Edge, Safari exports supported",

  // ── Empty states (v0.0.11 QoL slice 1) ──────────────────────────────────
  "empty.tree":                "This document has no folders yet.",
  "empty.list":                "This folder is empty.",
  "empty.list.description":    "Bookmarks added here will appear in this list.",
  "empty.search":              "No matches for '{query}'.",
  "empty.search.description":  "Try fewer search terms or relax the filters.",
  "empty.deadlinks.preRun":    "Click 'Check links' to start.",
  "empty.deadlinks.allGreen":  "All {count} links responded OK.",
  "empty.logs":                "No log entries yet.",

  // ── Loading shared (v0.0.11 QoL slice 1) ────────────────────────────────
  "loading.generic":           "Loading…",
};
