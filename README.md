# Lantern

A local-only Windows desktop app for exploring, sanitising, and re-exporting browser bookmark files.

[![CI](https://github.com/coatyl/lantern/actions/workflows/ci.yml/badge.svg)](https://github.com/coatyl/lantern/actions/workflows/ci.yml)
[![Licence](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](LICENSE)

---

## What is Lantern?

Lantern treats your exported `bookmarks.html` as a first-class document: the way a text editor treats `.txt`. It opens the file, lets you browse and clean it in a three-pane tree/list/detail view, previews every proposed change as a reviewable diff, and writes a clean copy to a new file. The original file is never modified.

It is built in Rust + Tauri 2 + React, ships as a small Windows executable, and is feature-frozen for v1.0 around bookmark hygiene: no live browser integration, no sync, no cloud.

## Why

Lantern is **local-only by default**. No telemetry, no analytics, no crash-reporting service, no update pings exist anywhere in the codebase. The single network-touching feature (the dead-link checker) is opt-in per session, off by default, and is omitted entirely from the offline build flavour at compile time. The "local-only" promise is taken seriously enough to be enforced in CI by a symbol-check that asserts no `reqwest`/`hyper` linkage in offline builds. Full details in [`SECURITY.md`](SECURITY.md).

That promise is the whole point. Bookmark exports leak more than people realise: tracking parameters, session tokens, account handles in URL paths, and embedded emails in titles. Lantern's job is to let you scrub those before you share, archive, or back up the file, without trusting the scrub to anyone else.

## Install

<!-- TODO: link to GitHub releases once signed binaries ship. -->
Download a signed Windows build from the releases page: *<TODO: link to GitHub releases once signed binaries ship>*.

Three flavours ship per release:

| Flavour | What it is |
|---|---|
| `lantern.exe` (portable) | Single executable. No installer, no registry writes. xcopy-deployable to a USB stick. |
| `lantern-installed.msix` | MSIX package for `winget` / Microsoft Store-style install. |
| `lantern-offline.exe` | Portable build with the dead-link checker compiled out: verifiably no networking code linked. |

Verify a signed download:

```powershell
Get-AuthenticodeSignature .\lantern.exe
```

The output should report `Status: Valid`. Unsigned dev builds and self-built binaries report `unsigned` in `Settings → About`.

### Build from source

Prerequisites: [Rust stable](https://rustup.rs/) (toolchain pinned in `rust-toolchain.toml`), [Node.js 20+](https://nodejs.org/), Microsoft C++ Build Tools, and WebView2 (pre-installed on Windows 11).

```powershell
git clone https://github.com/coatyl/lantern.git
cd lantern
npm install
npm run build
```

The build produces `target/release/lantern.exe` plus an NSIS installer when `bundle.active = true`. `target/` is gitignored; what a release is supposed to contain, and the hashes of the 2026-05-06 v1.0.0 copies, live in [`build/releases/README.md`](build/releases/README.md). For the offline-only flavour:

```powershell
cargo build --release -p lantern-app --no-default-features --locked
```

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the full developer setup, common commands, and project tour.

## Use

1. **Open** a `bookmarks.html` file (Ctrl+O), or a Chrome / Chromium `Bookmarks` JSON file from the profile directory. Firefox HTML already uses the same Netscape format.
2. **Browse** the folder hierarchy in the tree pane; the list pane shows the contents of the focused folder.
3. **Sanitise** a selection, a folder, or the whole document. Lantern proposes changes; you review them in a diff before anything is applied.
4. **Apply** the subset you approve. Undo restores the previous state byte-for-byte.
5. **Export** a clean copy to a new file. The original file is never written.
6. *(Opt-in)* run the dead-link checker over a selection. Off by default; clearly labelled when on.

The full keyboard map lives in `Settings → Keyboard`. The app is operable end-to-end from the keyboard alone (US-020).

## CLI

The `lantern-cli` crate builds a headless `lantern` binary that runs the same sanitisation passes from the command line. Useful for scripting, CI scrubs, or running on a machine without a desktop.

```powershell
# Print structural information about a bookmark file (counts, depth).
lantern info bookmarks.html

# Convert Chrome Bookmarks JSON (or Netscape HTML) to Netscape HTML.
lantern convert Bookmarks -o bookmarks.html

# Apply a built-in rule set and write the cleaned copy to a new file.
lantern sanitize bookmarks.html --rule-set full-scrub --output cleaned.html

# Preview the changes a rule set would make, without writing anything.
lantern sanitize bookmarks.html --rule-set full-scrub --dry-run

# Review exact-URL duplicates as proposed deletions (nothing is removed).
lantern sanitize bookmarks.html --rule-set find-duplicates --dry-run

# List the built-in rule sets (minimal-clean, aggressive-scrub, full-scrub,
# find-duplicates).
lantern rule-sets
```

`lantern --help` lists every subcommand and flag. The CLI shares the same core APIs as the desktop app, so a `sanitize` run from either produces identical output for the same rule set + input.

Without `--dry-run`, `lantern sanitize` **auto-approves every proposed change**, including destructive deletions from `find-duplicates`. Always inspect that rule set with `--dry-run` first. The GUI never auto-applies: each deletion stays unchecked until you approve it.

## Licence

Lantern is dual-licensed under [Apache 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option. See [`LICENSE`](LICENSE) for the top-level pointer. The chosen identifier is `Apache-2.0 OR MIT` (the standard Rust-ecosystem default), and it matches the literal reported by `Settings → About`.

## Security

Vulnerability reports go through [`SECURITY.md`](SECURITY.md). Please do not file security reports as public GitHub issues.

## Contributing

Lantern does not currently accept external pull requests. This is a pre-1.0 personal project under review for public release. Once the repo is flipped public, contributions follow the [`CONTRIBUTING.md`](CONTRIBUTING.md) guide: setup, common tasks, conventional commits, and PR template.
