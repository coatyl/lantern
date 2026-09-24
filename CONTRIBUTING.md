# Contributing to Lantern

How Lantern is built, tested and released. For what the app does, read the [README](README.md) first.

## Ground rules

- **Local-only is the product.** Network code lives only in `lantern-net`, behind the `checker` feature. A dependency anywhere else that pulls in an HTTP client (`reqwest`, `hyper`, `ureq`, a `fetch` polyfill) is rejected in review, and CI fails if `reqwest` or `hyper` reaches the offline build. Auto-update checks, crash reporters, analytics and "call home" URLs are not accepted.
- **Log the kind of operation and the count, never the content.** No URLs, titles, emails or usernames in log lines.
- **Write only where the user asked** (the export path) **or to the settings directory.** Anything else needs a reason in the PR.
- **Every change to a document is a proposal the user reviews.** Destructive changes start unapproved. The GUI and the CLI stay peers: a capability lands in `lantern-core` / `lantern-io` first.
- **Licence and sign-off.** Contributions are dual-licensed `Apache-2.0 OR MIT` (see [`LICENSE`](LICENSE)). The project uses the Developer Certificate of Origin rather than a CLA: sign commits with `git commit -s`.

## Setup

Windows 10 22H2+ or 11 is the primary platform:

1. Visual Studio Build Tools with the **Desktop development with C++** workload (the MSVC linker).
2. Rust via [rustup](https://rustup.rs/). `rust-toolchain.toml` pins stable with `rustfmt` and `clippy`.
3. Node.js at the version in `.nvmrc`.
4. WebView2 (already on Windows 11; otherwise install the Evergreen runtime).

Linux works for development once the Tauri WebKitGTK packages are installed (the list is in `.cursor/install.sh`). Only Windows builds are released, and the Rust CI jobs run on Windows.

```sh
git clone https://github.com/coatyl/lantern.git
cd lantern
npm ci          # root + ui workspaces, including the Tauri CLI
npm run dev     # Vite dev server + live-reloading Lantern window
```

The first `npm run dev` compiles the Rust backend and takes a few minutes. To work on the UI without Tauri, `npm run dev -w lantern-ui` serves it at <http://localhost:5173> with the IPC calls stubbed out (`ui/src/browser-stubs/`).

## Layout

```
crates/
  lantern-core/   parser (HTML + Chrome JSON), model, sanitize passes, diff, merge, search, emit. No I/O.
  lantern-io/     reading and writing bookmark files, settings, rule-set files.
  lantern-net/    the dead-link checker; the only crate allowed to touch the network.
  lantern-app/    Tauri shell: IPC commands (commands.rs), state, Tauri configs, icons.
  lantern-cli/    the `lantern-cli` binary.
ui/               React + Tailwind + Zustand front end (npm workspace `lantern-ui`).
  src/            shell/ (title bar, tabs, library home), panes/, components/, i18n/, ipc/.
  e2e/            Playwright specs, run against the browser stubs.
```

Keep `lantern-app` thin. If a command does real work, that work belongs in `lantern-core` or `lantern-io`.

## Everyday commands

All from the repo root.

| Command | What it runs |
|---|---|
| `npm run check` | `lint` then `test`: run this before pushing. |
| `npm run lint` | clippy (`-D warnings`), `cargo fmt --check`, `tsc --noEmit` on the UI. |
| `npm test` | Rust workspace tests, then Vitest. |
| `npm run test:rust` / `npm run test:ui` | Either half. |
| `npm run e2e` | Playwright; starts the Vite dev server itself. First run `npx -w lantern-ui playwright install chromium`. |
| `npm run fmt` | `cargo fmt --all`. |
| `npm run bindings` | Regenerate the ts-rs bindings in `ui/src/ipc/bindings/`. |
| `npm run build` | Release build of the app (`target/release/lantern.exe` + NSIS installer). |

Narrower runs: `cargo test -p lantern-core`, `cargo test <name-substring>`, `npm test -w lantern-ui -- --run TreePane`.

The CLI is the quickest way to exercise sanitisation without the UI:

```sh
cargo run -p lantern-cli -- sanitize crates/lantern-cli/tests/fixtures/small.html --rule-set aggressive-scrub --dry-run
cargo run -p lantern-cli -- convert crates/lantern-cli/tests/fixtures/chrome-bookmarks.json -o /tmp/bookmarks.html
```

Fuzz targets for the parser and URL treatments live in `crates/lantern-core/fuzz/` (nightly toolchain, run by hand; see its README).

## Common tasks

### Add a sanitisation treatment

1. Implement `Treatment` (`crates/lantern-core/src/sanitize/treatment.rs`) in the matching module under `crates/lantern-core/src/sanitize/treatments/`. `UtmTreatment` in `url_qp.rs` is a compact model. Use the `Change::set_field` family of constructors: they set `approved = !destructive`. Treatments that need the whole document, such as duplicate detection, implement `propose_document`.
2. Add unit tests in that module's `#[cfg(test)]` block, covering both matches and non-matches.
3. Re-export it from `treatments/mod.rs`, add an arm to `treatment_from_id` in `crates/lantern-io/src/ruleset.rs`, and add it to `builtin_treatment_catalogue` in `crates/lantern-app/src/commands.rs` so the rule-set editor offers it.
4. Built-in rule sets are defined in `builtin_rule_sets` in `crates/lantern-io/src/rulestore.rs`. Keep destructive treatments out of *Minimal clean*, *Aggressive scrub* and *Full scrub*; they get their own set, as *Find duplicates* does.
5. Add a line under `## [Unreleased]` in `CHANGELOG.md`.

### Add a Tauri command

1. Write the `#[tauri::command]` function in `crates/lantern-app/src/commands.rs`, with tests in the same file.
2. Register it in **both** `generate_handler!` lists in `crates/lantern-app/src/lib.rs` (with and without the `checker` feature), unless it is checker-only.
3. Put new IPC types in `crates/lantern-app/src/types.rs` and run `npm run bindings`. Mirror them in `ui/src/ipc/types.ts`, which the UI imports and which is maintained by hand.
4. Add the typed wrapper to `ui/src/ipc/index.ts`. If the browser preview or an e2e spec needs it, handle the command in `ui/src/browser-stubs/tauri-core.ts`.

### Change the UI

Colours are theme-aware CSS variables defined in `ui/src/index.css` and mapped in `ui/tailwind.config.js`. Use those tokens rather than raw colours. User-facing strings go in `ui/src/i18n/en.ts` (see `ui/src/i18n/README.md` for adding a locale). The app must stay usable from the keyboard alone.

## Branches

One branch per pull request, named for what it changes:

| Kind | Pattern | Examples |
|---|---|---|
| Feature | `feat/<scope>-<topic>` | `feat/io-firefox-places`, `feat/ui-review-filters` |
| Fix | `fix/<scope>-<topic>` | `fix/core-undo-order`, `fix/ui-light-theme-contrast` |
| Other work | `<type>/<topic>` using the commit types below | `docs/contributing-branches`, `ci/linux-first`, `refactor/app-commands` |
| Release preparation | `release/<version>` | `release/0.2.0` |

- Lowercase, words joined with `-`, no more than about five words. The scope is one of the commit scopes below; leave it out when the change spans several.
- No personal, random or tool-generated names (`wyvern/…`, `agent-branch/…`, `patch-1`): the name should tell a reviewer what the branch is before they open it.
- Delete the branch once its pull request is merged or closed. Turn on **Settings → General → Automatically delete head branches** so GitHub does it.
- `main` is the only long-lived branch.

## Commits and pull requests

- [Conventional Commits](https://www.conventionalcommits.org/): `<type>(<scope>): <subject>`. Types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `chore`, `revert`. Scopes: `core`, `io`, `net`, `app`, `cli`, `ui`, `docs`, `ci`, `release`. Breaking changes use `!` plus a `BREAKING CHANGE:` footer.
- Rebase on `main` rather than merging it in, and expect a squash merge.
- Open an issue before starting a feature so scope can be agreed.
- Every PR description includes a **Privacy impact** line: does it touch the network, the filesystem, logging or user data? Write "none" if not.
- New behaviour needs tests, bug fixes need a regression test, and user-visible changes need a `CHANGELOG.md` entry.

## CI

`.github/workflows/ci.yml` runs on every PR and on pushes to `main`:

| Job | Checks |
|---|---|
| `rust-fmt-clippy` | `cargo fmt --check`, clippy with `-D warnings` |
| `rust-test` | `cargo test --workspace` |
| `ui-lint-typecheck-unit` | `tsc` and Vitest |
| `ui-e2e` | Playwright (non-blocking) |
| `offline-build-symbol-check` | the `--no-default-features` build contains no `reqwest` or `hyper` |
| `build-portable-x64` | builds and uploads the portable exe |
| `sign-windows-installed` | signed NSIS/MSI/exe; runs only when the certificate secret is set |
| `reproducible-build-check` | two clean builds hash identically (non-blocking) |

The build, e2e and signing jobs are skipped when a change touches only Markdown files.

## Releasing

`.github/workflows/release.yml` builds the portable, offline, NSIS and MSI artefacts on `windows-latest` and publishes them, with `SHA256SUMS.txt`, as a GitHub Release.

1. Set the new version in `Cargo.toml`, `package.json`, `ui/package.json` and `crates/lantern-app/tauri.conf.json`, then refresh the lockfiles (`cargo update -w`, `npm install`).
2. In `CHANGELOG.md`, rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD`, open a new empty `## [Unreleased]`, and update the compare links at the bottom.
3. Merge to `main`, then either push the tag (`git tag -a vX.Y.Z -m "Lantern vX.Y.Z" && git push origin vX.Y.Z`) or run the **Release** workflow by hand on `main`, which creates the tag.

If a tag was pushed while Actions could not run, run **Release** by hand with the `tag` input set to it (e.g. `v0.1.0`).

The workflow refuses to publish if the tag does not match all four version files or if `CHANGELOG.md` has no section for the version. That section becomes the release notes. Artefacts are unsigned until a code-signing certificate is available.

## Troubleshooting

- **Linker errors on Windows:** the MSVC toolchain is missing. Reinstall the C++ workload, and check that `rustup show` reports `stable-x86_64-pc-windows-msvc`.
- **Path-length errors in tests on Windows:** run `git config --global core.longpaths true` and enable `LongPathsEnabled` in the registry.
- **Clippy passes locally but fails in CI:** CI uses the latest stable. Run `rustup update`.

Security reports go through [`SECURITY.md`](SECURITY.md), not the issue tracker.
