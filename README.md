# Lantern

A local-only Windows desktop app for exploring, sanitising, and re-exporting browser bookmark files.

[![CI](https://github.com/coatyl/lantern/actions/workflows/ci.yml/badge.svg)](https://github.com/coatyl/lantern/actions/workflows/ci.yml)
[![Licence](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](LICENSE)

Lantern treats a bookmark export the way a text editor treats a `.txt` file. It opens the file, lets you browse and clean it, shows every proposed change for review before anything is applied, and writes a clean copy to a new file. The file you opened is never modified.

Built with Rust, Tauri 2 and React.

## Privacy

Bookmark exports leak more than people expect: tracking parameters, session tokens, account handles in URL paths, email addresses in titles. Lantern lets you scrub them before you share, archive or back up the file, without handing the file to anyone.

Lantern is local-only. It has no telemetry, analytics, crash reporting or update checks. The only feature that touches the network is the dead-link checker, which stays off until you enable it in **Settings → General**. The offline build leaves it out at compile time, and CI fails if `reqwest` or `hyper` appear in that build's dependency graph. See [`SECURITY.md`](SECURITY.md).

## Install

Download a Windows x64 build from the [releases page](https://github.com/coatyl/lantern/releases). Each release has:

| File | What it is |
|---|---|
| `lantern-v<ver>-portable-x64.exe` | Single executable, no installer. |
| `lantern-v<ver>-offline-x64.exe` | Portable build with the dead-link checker compiled out: no networking code linked. |
| `lantern-v<ver>-installer-nsis-x64.exe` | Per-user NSIS installer. |
| `lantern-v<ver>-installer-x64.msi` | MSI installer for managed deployment. |
| `SHA256SUMS.txt` | SHA-256 of every file above. |

Check a download against `SHA256SUMS.txt`:

```powershell
(Get-FileHash -Algorithm SHA256 .\lantern-v0.2.0-portable-x64.exe).Hash
```

Settings and rule sets live in `%APPDATA%\Lantern`. To keep them beside a portable copy instead, put an empty `settings.toml` next to the executable.

**Releases are not code-signed yet.** SmartScreen warns on first run, and **Settings → About** reports `unsigned`.

## Use

1. **Open** a file with Ctrl+O or from the library home, which lists recent files. Lantern reads Netscape bookmark HTML (the export format of Chrome, Edge, Firefox and Safari) and Chrome / Chromium `Bookmarks` JSON straight from a browser profile.
2. **Browse** the folder tree, the list of the focused folder, and the details of the selected item. Several files can be open in tabs.
3. **Run a rule set** from the detail pane on the whole document or just the focused folder. Built-in sets: *Minimal clean*, *Aggressive scrub*, *Full scrub*, and *Find duplicates*, which proposes deleting exact-URL duplicates. The rule-set editor lets you build your own.
4. **Review** the proposed changes. They take over the main area: one card per bookmark or folder with its path and a before/after diff, filters by kind of change, and bulk selection. Destructive changes, including deletions, start unselected. Ctrl+Enter applies, Escape discards, and Ctrl+Z undoes an apply.
5. **Export** a clean copy to a new HTML file. Lantern refuses to overwrite the file it opened.

**Tools** in the title bar compare two tabs, merge documents, and run the dead-link checker once it is enabled. Ctrl+K opens a command palette, and **Settings → Keyboard** lists every shortcut.

## CLI

`lantern-cli` runs the same rule sets without the GUI. It is not in the release downloads; build it from source (below) with `cargo build --release -p lantern-cli`.

```powershell
# Counts and depth of a bookmark file (HTML or Chrome JSON).
lantern-cli info bookmarks.html

# Chrome Bookmarks JSON (or Netscape HTML) to Netscape HTML.
lantern-cli convert Bookmarks -o bookmarks.html

# Apply a built-in rule set; writes a cleaned copy.
lantern-cli sanitize bookmarks.html --rule-set full-scrub --output cleaned.html

# Show what a rule set would change, without writing anything.
lantern-cli sanitize bookmarks.html --rule-set find-duplicates --dry-run

# List the built-in rule sets.
lantern-cli rule-sets
```

Without `--output`, `sanitize` writes `<input>.clean.html`. `--rule-set-file` takes a `.lantern-rules.toml` instead of a built-in name. `lantern-cli --help` lists every flag.

Without `--dry-run`, `sanitize` **applies every proposed change**, including the deletions proposed by `find-duplicates`. Run it with `--dry-run` first. The GUI never auto-applies deletions.

## Build from source

You need [Rust stable](https://rustup.rs/) (`rust-toolchain.toml` selects it), Node.js 20 (see `.nvmrc`), the Microsoft C++ Build Tools, and WebView2, which ships with Windows 11.

```powershell
git clone https://github.com/coatyl/lantern.git
cd lantern
npm ci
npm run build
```

That builds `target/release/lantern.exe` and an NSIS installer under `target/release/bundle/nsis/`. The release flavours are built the same way with the configs in `crates/lantern-app/`. For example, the offline flavour:

```powershell
npx tauri build --config crates/lantern-app/tauri.offline.conf.json -- --no-default-features
```

[`CONTRIBUTING.md`](CONTRIBUTING.md) covers development setup, tests and the project layout.

## Licence

Dual-licensed under [Apache 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option (`Apache-2.0 OR MIT`; see [`LICENSE`](LICENSE)).

## Security

Report vulnerabilities as described in [`SECURITY.md`](SECURITY.md), not in public issues.

## Contributing

Issues and pull requests are welcome. [`CONTRIBUTING.md`](CONTRIBUTING.md) covers setup, the checks to run before a PR, and conventions. Security reports go through [`SECURITY.md`](SECURITY.md), not public issues.
