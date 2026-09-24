# Lantern Roadmap

This file tracks concrete work. The identity-level bets, and the non-negotiables every change must respect, are in [`PHILOSOPHY.md`](PHILOSOPHY.md). Nothing here is a commitment or a date.

## 0.2.0 — the warm-archive release (in progress)

Landed on the 0.2 line; see `CHANGELOG.md` → `[Unreleased]` for detail.

- A theme-aware design system; light mode is readable again.
- A full-width review surface for proposed changes, with folder-scoped passes.
- A one-row workspace toolbar, search as you type, and a keyboard- and screen-reader-friendly folder tree.
- Saving always writes a copy; closing edited tabs asks first; drag-and-drop to open.
- Chrome `Bookmarks` JSON import, `lantern-cli convert`, the **Find duplicates** rule set, and the command palette.
- A codebase cleanup: `commands.rs` split per domain, a shared modal shell, generated IPC types used directly, and fixes to undo order, node-id reuse, substring search and rule-set duplication.

Left before tagging: CI green on Windows (blocked on GitHub Actions starting jobs), and a manual pass in the real Tauri shell (drag-and-drop, window close, file dialogs), since those paths only run in the desktop build.

## 0.3.0 — every format, smarter curation (next)

- **Writers:** JSON (Chrome-compatible), Markdown link lists and CSV, through `lantern-cli convert --to <format>` and a format picker in the save-copy dialog.
- **Readers:** Firefox `places.sqlite` (read-only, copied before reading so a running Firefox is never touched), Safari `Bookmarks.plist`.
- **Near-duplicates** *(landed)*: URLs that differ only by tracking parameters, fragment, `www.`, trailing slash, parameter order or `http`/`https`, proposed as reviewable deletions that name the kept bookmark and what differs.
- **Library management:** remove a volume from the library, reveal it in the file manager, show each volume's size and last-opened time.
- **Folder health:** empty folders, single-item folders and very deep nesting, surfaced as proposals in the review surface.
- **CLI parity:** `lantern-cli dedupe`, rule sets loaded from disk by name, and `--json` output for scripting.

## Before 1.0

- **Code signing.** The CI signing job is ready but has no Authenticode certificate. Once one exists, move its signtool step into `release.yml`.
- **MSIX packaging.** Deferred until publisher-identity signing and Tauri's MSIX support are in place.
- **Windows ARM64.** No build target yet.
- **Accessibility.** A manual NVDA screen-reader pass has not been done.
- **CI hardening.** `ui-e2e` and `reproducible-build-check` should become blocking once they are reliably green.
- **CLI parity.** Custom rule-set discovery from disk, plus `merge` and `diff` subcommands for what the GUI already does.
- **Locales.** English is the only one; see `ui/src/i18n/README.md`.

## Longer-term directions

Each of these spans several releases. Each lands as a `lantern-core` / `lantern-io` capability with a CLI verb and a reviewable diff, and works offline.

1. **More platforms** ([Bet 3](PHILOSOPHY.md)). The workspace already builds on Linux, so the first step is to ship Linux and macOS from release CI. Mobile needs `lantern-io`'s settings, recent-files and recovery paths behind a platform trait, a document-picker backend, and a stacked layout. The real costs are webview parity (WKWebView, WebKitGTK) and iOS sandboxing.
2. **More formats** ([Bet 6](PHILOSOPHY.md)). Done so far: the Chrome / Chromium JSON reader and `lantern-cli convert`. Next readers: Firefox `places.sqlite`, Safari plist, and Pocket, Raindrop and OneTab exports. Next writers: JSON, Markdown, CSV and a lossless native format. Live profile reads stay read-only and are clearly labelled.
3. **Curation, not just scrubbing** ([Bet 2](PHILOSOPHY.md)). Done so far: exact- and near-duplicate review. Next: stale-save hints from timestamps, and on-device folder suggestions, all proposed as diffs. Opt-in link intelligence, such as redirect flattening, stays in `lantern-net`.
4. **Sandboxed treatment packs** ([Bet 5](PHILOSOPHY.md)). Third-party treatments run as WebAssembly components with no I/O: a node goes in, proposed `Change`s come out. They are distributed as signed, file-based packs that are imported from disk and toggled in the rule-set editor. The open questions are ABI stability and a trust UX that is more than "click OK".
5. **A versioned local library** ([Bet 1](PHILOSOPHY.md)). Content-addressed snapshots in `lantern-io`, a 3-way merge built on the diff engine, and optional end-to-end-encrypted sync to a folder the user chooses. No Lantern account or server.
