# Lantern Roadmap

This file tracks concrete work. The identity-level bets, and the non-negotiables every change must respect, are in [`PHILOSOPHY.md`](PHILOSOPHY.md). Nothing here is a commitment or a date.

## Before 1.0

- **Code signing.** The CI signing job is ready but has no Authenticode certificate. Once one exists, move its signtool step into `release.yml`.
- **MSIX packaging.** Deferred until publisher-identity signing and Tauri's MSIX support are in place.
- **Windows ARM64.** No build target yet.
- **Accessibility.** A manual NVDA screen-reader pass has not been done.
- **CI hardening.** `ui-e2e` and `reproducible-build-check` should become blocking once they are reliably green.
- **CLI parity.** Custom rule-set discovery from disk, plus `merge` and `diff` subcommands for what the GUI already does.
- **Locales.** English is the only one; see `ui/src/locales/README.md`.

## Longer-term directions

Each of these spans several releases. Each lands as a `lantern-core` / `lantern-io` capability with a CLI verb and a reviewable diff, and works offline.

1. **More platforms** ([Bet 3](PHILOSOPHY.md)). The workspace already builds on Linux, so the first step is to ship Linux and macOS from release CI. Mobile needs `lantern-io`'s settings, recent-files and recovery paths behind a platform trait, a document-picker backend, and a stacked layout. The real costs are webview parity (WKWebView, WebKitGTK) and iOS sandboxing.
2. **More formats** ([Bet 6](PHILOSOPHY.md)). Done so far: the Chrome / Chromium JSON reader and `lantern-cli convert`. Next readers: Firefox `places.sqlite`, Safari plist, and Pocket, Raindrop and OneTab exports. Next writers: JSON, Markdown, CSV and a lossless native format. Live profile reads stay read-only and are clearly labelled.
3. **Curation, not just scrubbing** ([Bet 2](PHILOSOPHY.md)). Done so far: exact-URL duplicate review. Next: near-duplicates that differ only in query or fragment, stale-save hints from timestamps, and on-device folder suggestions, all proposed as diffs. Opt-in link intelligence, such as redirect flattening, stays in `lantern-net`.
4. **Sandboxed treatment packs** ([Bet 5](PHILOSOPHY.md)). Third-party treatments run as WebAssembly components with no I/O: a node goes in, proposed `Change`s come out. They are distributed as signed, file-based packs that are imported from disk and toggled in the rule-set editor. The open questions are ABI stability and a trust UX that is more than "click OK".
5. **A versioned local library** ([Bet 1](PHILOSOPHY.md)). Content-addressed snapshots in `lantern-io`, a 3-way merge built on the diff engine, and optional end-to-end-encrypted sync to a folder the user chooses. No Lantern account or server.
