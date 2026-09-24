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
  "statusBar.bookmarks.one":   "1 bookmark",
  "statusBar.folders":         "{n} folders",
  "statusBar.folders.one":     "1 folder",
  "statusBar.separators":      "{n} separators",
  "statusBar.separators.one":  "1 separator",
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
  "commandPalette.saveCopy":     "Save copy",
  "commandPalette.saveCopyAs":   "Save copy as…",
  "commandPalette.runPass":      "Run pass",
  "commandPalette.search":       "Focus search",
  "commandPalette.toggleTheme":  "Toggle theme",

  // ── Files (open / save a copy / undo) ────────────────────────────────────
  "file.dropToOpen":           "Drop bookmark files to open them",
  "file.unsupported":          "{name} isn’t a bookmark file Lantern can open (HTML export or Chrome Bookmarks JSON).",
  "file.openFailed":           "Couldn’t open {name}: {reason}",
  "file.saved":                "Saved a copy to {name}.",
  "file.saveFailed":           "Couldn’t save the copy: {reason}",
  "file.refuseOriginal":       "Lantern never overwrites the file you opened. Choose another name for the copy.",
  "file.nothingToUndo":        "Nothing to undo.",
  "file.nothingToRedo":        "Nothing to redo.",
  "titleBar.saveCopy":         "Save copy…",
  "titleBar.saveCopyHint":     "Save a clean copy (Ctrl+S). The original file is never changed.",

  // ── Tree pane ────────────────────────────────────────────────────────────
  "tree.label":                "Folders",
  "tree.unnamed":              "(unnamed)",

  // ── List pane ────────────────────────────────────────────────────────────
  "list.root":                 "All bookmarks",
  "list.search":               "Search this document",
  "list.search.placeholder.substring": "Search titles and URLs…",
  "list.search.placeholder.glob":      "Glob, e.g. *github.com/*",
  "list.search.placeholder.regex":     "Regular expression…",
  "list.search.clear":         "Clear search",
  "list.search.results":       "{n} results for “{query}”",
  "list.search.results.one":   "1 result for “{query}”",
  "list.search.invalid":       "That pattern isn’t valid yet.",
  "list.search.mode":          "Match",
  "list.search.mode.substring": "Text",
  "list.search.mode.glob":     "Glob",
  "list.search.mode.regex":    "Regex",
  "list.search.titles":        "Titles",
  "list.search.urls":          "URLs",
  "list.search.back":          "Back to folder",
  "list.filter":               "Filter",
  "list.filter.active":        "Filter active",
  "list.new":                  "New",
  "list.new.bookmark":         "Bookmark",
  "list.new.folder":           "Folder",
  "list.new.separator":        "Separator",
  "list.count.items":          "{n} items",
  "list.count.items.one":      "1 item",
  "list.count.results":        "{n} results",
  "list.count.results.one":    "1 result",

  // ── Detail pane ──────────────────────────────────────────────────────────
  "detail.scope":              "Run on",
  "detail.scope.document":     "Whole document",
  "detail.scope.folder":       "This folder",

  // ── Review (proposed changes) ──────────────────────────────────────────
  "review.region":             "Proposed changes",
  "review.title":              "Review changes",
  "review.meta":               "{ruleSet} · {scope} · {n} proposed changes",
  "review.meta.one":           "{ruleSet} · {scope} · 1 proposed change",
  "review.selected":           "{n} of {total} selected",
  "review.discard":            "Discard",
  "review.apply":              "Apply {n} changes",
  "review.apply.one":          "Apply 1 change",
  "review.applyWithDeletes":   "Apply {n} changes ({deletions})",
  "review.applyWithDeletes.one": "Apply 1 change ({deletions})",
  "review.deletions":          "{n} deletions",
  "review.deletions.one":      "1 deletion",
  "review.applying":           "Applying…",
  "review.applyHint":          "Apply the selected changes (Ctrl+Enter)",
  "review.applied":            "Applied {n} changes.",
  "review.applied.one":        "Applied 1 change.",
  "review.undo":               "Undo",
  "review.kindFilter":         "Filter by kind of change",
  "review.kind.all":           "All",
  "review.kind.url":           "URL",
  "review.kind.title":         "Title",
  "review.kind.folder_name":   "Folder name",
  "review.kind.node":          "Delete",
  "review.filterPlaceholder":  "Filter changes…",
  "review.selectAll":          "Select all",
  "review.clearAll":           "Clear all",
  "review.selectShown":        "Select shown",
  "review.clearShown":         "Clear shown",
  "review.noMatches":          "No changes match this filter.",
  "review.untitled":           "Untitled",
  "review.destructive":        "destructive",
  "review.deleteNode":         "Remove this item from the document.",
  "review.nothing":            "Nothing to change: {scope} is already clean under {ruleSet}.",
  "review.scope.document":     "Whole document",
  "review.scope.folder":       "Folder “{name}”",

  // ── Library home (the stacks; the workspace is inside a volume) ─────────
  "library.region":                    "Library",
  "library.title":                     "Library",
  "library.subtitle":                  "A private collection of bookmark files on this machine.",
  "library.empty.title":               "Your library is empty",
  "library.empty.description":         "Open a bookmark file to start a private collection. Files you open stay listed here.",
  "library.open":                      "Open file…",
  "library.recentHeading":             "Recent volumes",
  "library.recentCount":               "{n} volumes",
  "library.recentCount.one":           "1 volume",
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
  "library.hint":                      "Drop a bookmark file anywhere to open it · Ctrl+S saves a clean copy, never the original · Chrome, Firefox, Edge and Safari exports and Chrome Bookmarks JSON",
};
