# Contributing to Lantern

Welcome. This guide is for anyone who wants to help build Lantern, whether that is fixing a typo in the docs, adding a new sanitization treatment, or reviewing a tricky pull request. It is written with the assumption that you have used Git and know your way around a terminal, but not that you already know Rust, Tauri, or React.

This is the friendly walkthrough: setup, common tasks, and PR expectations. The formal engineering process, PRD, technical design, and testing plan live in the project's private docs and are shared with maintainers on request.

---

## 1. Before you start

### 1.1 What kind of contribution?

There are a few common shapes:

- **A bug report.** File an issue; you don't need to write code.
- **A documentation fix.** Small PRs are welcome and reviewed quickly.
- **A new feature.** Open an issue first so we can discuss scope. Features that land without prior discussion often get sent back for rework, and nobody enjoys that.
- **A new sanitization treatment.** These are the single most valuable kind of contribution Lantern can get. See §8.1 for a walkthrough.
- **A reviewer's eye.** Experienced Rust or Tauri developers reviewing open PRs is enormously helpful even if you never write a line.

### 1.2 Code of conduct

We ask every contributor to behave respectfully toward others. Disagreements about technical decisions are fine and encouraged; personal attacks, harassment, and dismissive behavior are not. Concerns can be raised privately with a maintainer.

### 1.3 Licensing and sign-off

Contributions are licensed under the project's main license (see the `LICENSE` file at the repo root). By opening a pull request, you assert that you have the right to contribute the code and agree to that license.

Lantern uses the **Developer Certificate of Origin** rather than a CLA. Every commit must include a `Signed-off-by:` line, added automatically by `git commit -s`. This is a simple way of saying "I wrote this, or I have the right to submit it."

## 2. Setting up your environment

Lantern is Windows-first. You can develop on Windows 10 (22H2+) or Windows 11. Building and running the app on Linux or macOS is possible for some of the Rust crates (notably `lantern-core` and `lantern-cli`), but you cannot run the full app outside Windows; WebView2 is Windows-only.

### 2.1 Prerequisites

Install, in this order:

1. **Visual Studio Build Tools.** From the Visual Studio installer, select the "Desktop development with C++" workload. The Rust toolchain requires MSVC's linker.
2. **Rust.** Install via `rustup` from https://rustup.rs/. After install, run:
   ```
   rustup component add rustfmt clippy
   ```
   The repository pins its toolchain in `rust-toolchain.toml`; `rustup` picks that version automatically when you `cd` into the repo; no need to install it manually.
3. **Node.js.** Use the version in `.nvmrc`. Install via `nvm-windows` or a direct download. Run `npm --version` to confirm.
4. **WebView2 runtime.** Already present on Windows 11 and recent Windows 10. If missing, install Evergreen Bootstrapper from Microsoft.
5. **Tauri CLI.** Installed automatically as part of the root npm dependencies; no separate install needed. After the next step (`npm ci`), the CLI is available as `npx tauri`.

### 2.2 Recommended but optional utilities

These make life nicer but are not required to build or run the app:

```
cargo install cargo-llvm-cov --locked   # coverage reports
cargo install cargo-nextest --locked    # faster test runner
cargo install cargo-deny --locked       # dependency policy checks
cargo install cargo-audit --locked      # advisory checks
```

### 2.3 Clone and first build

```
git clone https://github.com/coatyl/lantern.git
cd lantern
cargo fetch                 # downloads Rust deps
npm install                 # installs npm-workspaces (root + ui together)
npm run dev
```

The last command builds the Rust backend, starts the Vite dev server for the UI, and opens a live-reloading Lantern window. The first `tauri dev` takes a few minutes; subsequent runs are fast because of incremental compilation.

If something goes wrong, the troubleshooting section in §10 covers the most common issues.

### 2.4 Editor setup

Any editor works. The maintainer set currently uses:

- **VS Code** with the `rust-analyzer`, `tauri-vscode`, `ESLint`, and `Tailwind CSS IntelliSense` extensions.
- **Zed** with its built-in Rust and TypeScript support.

A workspace settings file `.vscode/settings.json` is committed with sensible defaults.

### 2.5 Build output (`target/` and `build/releases`)

`target/` is Cargo and Tauri output at the repo root. Ignore it; do not commit it. Debug builds land in `target/debug/`, release builds and `lantern.exe` in `target/release/`, and bundled installers in `target/release/bundle/` when bundling is on.

`build/releases/` is the written record of ship artefacts, not a second copy of `target/`. It names the files, sizes and SHA256 hashes for the 2026-05-06 v1.0.0 set that used to sit in `target/lantern-v1.0.0/`. Read [`build/releases/README.md`](build/releases/README.md) before treating any local `target/` folder as the release.

The live clone URL while the repo is on Cursor Origin:

```
git clone https://origin.cursor.com/john-pork-corp/lantern.git
```

The `github.com/coatyl/lantern` URL is the canonical GitHub remote; the Cursor Origin URL above mirrors it while the repo is hosted there.

## 3. Project tour

Before you change anything, it helps to know where things live.

```
lantern/
├── crates/
│   ├── lantern-core/     ← pure logic: parser, model, sanitize, diff, emit
│   ├── lantern-io/       ← filesystem: read bookmark files, settings, rule sets
│   ├── lantern-net/      ← the ONLY crate that touches the network (feature-gated)
│   ├── lantern-app/      ← Tauri glue: commands, state, events, conf, icons
│   └── lantern-cli/      ← optional headless CLI (deferred to v0.1.0)
├── ui/                   ← React + Tailwind UI (npm workspace member)
├── build/releases/       ← hashes and notes for ship artefacts; not the binaries
├── fixtures/             ← real bookmark exports for tests
└── CONTRIBUTING.md       ← this guide
```

A few rules to keep in mind:

- `lantern-core` has no I/O. Don't add any. If you need to read a file, your code belongs in `lantern-io`.
- `lantern-net` is the one place network code can live. Adding a crate dependency that transitively pulls in a network client anywhere else will fail CI.
- `lantern-app` is thin glue. If a Tauri command is doing real work, most of that work belongs in `core` or `io` and the command is a wrapper.

A longer architectural tour lives in the project's internal Technical Design Document (private/docs/); ask a maintainer if you need access.

## 4. Running things locally

### 4.1 The app

```
npm run dev
```

This runs from the repo root. It starts the Vite dev server for the UI and builds + opens a live-reloading Lantern window. Changes to the UI hot-reload; changes to the Rust backend trigger an automatic rebuild and window restart.

### 4.2 Tests

All from the repo root:

```
npm test                                       # full suite (Rust + UI)
npm run test:rust                              # Rust workspace only
npm run test:ui                                # Vitest only

cargo test -p lantern-core                     # one Rust crate
cargo test -p lantern-core --test sanitize     # one test file
cargo test url_qp_utm                          # by test name substring

npm test -w lantern-ui -- TreePane             # one UI component
```

### 4.3 Linters and formatters

Run these before opening a PR. CI will run them anyway, but it is faster to catch issues locally.

```
npm run lint                                   # full lint (clippy + fmt --check + tsc)
npm run lint:rust                              # clippy --workspace --all-targets -D warnings
npm run lint:fmt                               # cargo fmt --check
npm run lint:ts                                # tsc --noEmit -p ui

npm run fmt                                    # cargo fmt --all (apply)
npm run check                                  # lint + test (the pre-commit gate)
```

### 4.4 The CLI

```
cargo run -p lantern-cli -- --help
cargo run -p lantern-cli -- sanitize crates/lantern-cli/tests/fixtures/small.html \
  --rule-set aggressive-scrub \
  --output /tmp/out.html
```

The CLI is a good way to work on sanitization treatments without bouncing through the UI. It has the same core APIs under the hood.

## 5. Making a change

### 5.1 The flow

1. **Find or open an issue.** For anything non-trivial, an issue exists before a PR does.
2. **Fork** (if external) or **branch** (if you have push access). Branch name: `<type>/<slug>`, e.g. `feat/url-affiliate-treatment`.
3. **Code.** Run tests and linters frequently.
4. **Commit** using conventional commits (see §6).
5. **Open a PR** against `main` with the PR template filled in, especially the **Privacy impact** field. See §7.
6. **Respond to review.** Expect one round of comments for small changes, two to three for larger ones.
7. **Squash-merge.** Maintainers handle the merge button.

### 5.2 Keeping your branch current

Rebase, don't merge. This keeps your branch's history linear on top of `main`'s, which is what the squash-merge policy expects. A linear history makes `git bisect` and changelog generation straightforward; both are real workflows in this project.

```
git fetch origin
git rebase origin/main
```

If you get conflicts, resolve them and continue:

```
git rebase --continue
```

If you get stuck, ask. Rebasing is a skill and nobody expects you to be an expert.

## 6. Commit messages

Lantern uses [Conventional Commits](https://www.conventionalcommits.org/). The process doc §4 has the full spec. The short version:

```
<type>(<scope>): <subject>

<optional body>

<optional footer>
```

- **type:** `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `chore`, `revert`.
- **scope:** `core`, `io`, `net`, `app`, `cli`, `ui`, `docs`, `ci`, `fixtures`, `release`.
- **subject:** imperative, lowercase, no trailing period, ≤ 72 chars.
- **breaking changes:** `feat(core)!: ...` and a `BREAKING CHANGE:` footer.

Examples:

```
feat(core): add url.qp.affiliate treatment for ebay

Strips the ebay campid and icep parameters alongside the existing
amazon tag handling. Host-scoped to ebay.* so other hosts are not
affected.

Refs #142
```

```
fix(io): prevent portable build from writing to %APPDATA%

The portable flavor was falling back to the installed path when the
binary's folder was read-only. The correct behavior is to prompt the
user for a writable settings path at first launch.

Closes #203
```

If you are not sure what type or scope to use, pick the closest one. Reviewers can suggest a better choice during the PR.

## 7. Pull requests

### 7.1 The template

Every PR opens with a description filled in from the template. The fields are:

- **Summary.** What changes, in plain language.
- **Motivation.** Why. Link to the issue, PRD section, or ADR.
- **Implementation notes.** Anything a reviewer should know.
- **Privacy impact.** Does this touch network, filesystem, logging, or user data? **This field is not optional.** If the answer is "none," write "none"; do not delete the field.
- **Testing.** What you tested, what CI covers, anything manual.
- **Checklist.** A set of boxes for linters, tests, docs.

### 7.2 What reviewers look for

- **Correctness.** Does it do what the description says?
- **Tests.** New behavior has tests. Bugs come with regression tests.
- **Privacy.** Does any new code add a network call, a log line, a file write, or touch user data? These get extra scrutiny.
- **Style.** Formatting, naming, idiom.
- **Docs.** User-visible changes update the changelog and relevant docs.

Reviewers use four prefixes on their comments:

- `nit:` a minor style thing. Non-blocking.
- `question:` I don't understand this. Non-blocking.
- `suggestion:` here is an alternative. Non-blocking.
- `required:` this needs to change before merge. **Blocking.**

Only `required:` comments block merge. If you receive only `nit:` and `suggestion:` comments, you can choose to address them or politely decline.

### 7.3 After review

Push changes to the same branch (no force-push is needed for most reviews; rebase only when necessary). Reply to each `required:` comment explaining what you did. Don't mark threads resolved; the reviewer will.

## 8. Common tasks

Worked examples for the things contributors do most often.

### 8.1 Adding a sanitization treatment

This is the highest-impact contribution type. Let's walk through adding a hypothetical `url.qp.custom_shop_tracker` treatment that strips a specific shop's session tracking parameters.

1. **Find the right file.** URL query-parameter treatments live in `crates/lantern-core/src/sanitize/treatments/url_qp.rs`.
2. **Write the struct.**
   ```rust
   pub struct StripCustomShopTracker;

   impl Treatment for StripCustomShopTracker {
       fn id(&self) -> &'static str { "url.qp.custom_shop_tracker" }
       fn name(&self) -> &'static str { "Strip CustomShop tracker params" }
       fn category(&self) -> TreatmentCategory { TreatmentCategory::UrlQueryParam }
       fn is_destructive(&self) -> bool { false }

       fn propose(&self, node: &Node, _ctx: &PassContext) -> Vec<Change> {
           let Node::Bookmark(b) = node else { return vec![]; };
           let mut url = b.url.clone();
           let removed = strip_params(&mut url, &["cs_session", "cs_uid", "cs_track"]);
           if removed.is_empty() {
               vec![]
           } else {
               vec![Change {
                   node_id: b.id,
                   field: Field::Url,
                   before: b.url.to_string(),
                   after: url.to_string(),
                   treatment_id: self.id(),
                   rationale: format!("Strips {}.", removed.join(", ")).into(),
                   destructive: false,
                   approved: true,
               }]
           }
       }
   }
   ```
3. **Register it.** Add it to the built-in treatment registry in `treatments/mod.rs`, then add a `treatment_from_id` arm in `crates/lantern-io/src/ruleset.rs` (and the GUI catalogue in `lantern-app` if the picker should list it). Document-level treatments such as `structure.duplicates.exact_url` implement `propose_document` and belong in their own rule set when they delete nodes — do not add destructive deletes to Minimal / Aggressive / Full.
4. **Write tests.** Create `treatments/tests/url_qp_custom_shop_tracker.rs` with a list of matches and non-matches.
   ```rust
   #[test]
   fn strips_session_params() {
       let t = StripCustomShopTracker;
       assert_strips(&t,
           "https://shop.example.com/item?cs_session=abc&cs_uid=123",
           "https://shop.example.com/item");
   }
   ```
5. **Update the treatment catalog in the PRD.** §8.3.2 of `01-PRD.md` lists every built-in treatment.
6. **Add a changelog entry** under `[Unreleased]` → `Added`.
7. **Open the PR.** Privacy impact: "New treatment. No new I/O or network code."

### 8.2 Adding a Tauri command

Commands live in `crates/lantern-app/src/commands/`. They are thin wrappers around core logic.

1. Add the command function with the `#[tauri::command]` attribute.
2. Register it in `app.rs`'s `generate_handler![]` macro.
3. Add the corresponding TypeScript wrapper in `ui/src/ipc/`.
4. Add a test in `crates/lantern-app/tests/`.

Type sharing is automatic via `ts-rs`; run `cargo test -p lantern-app export_bindings` to regenerate the TypeScript bindings into `ui/src/ipc/bindings/`.

### 8.3 Adding a UI component

Components live under `ui/src/` grouped by area (see §3). Use Tailwind classes that map to the design tokens. If you find yourself reaching for a color or font that is not in `ui/src/tokens/`, stop; changes to the token set are a design decision and should go through the design doc.

Prefer composing shadcn primitives over building from scratch. Prefer Radix for low-level accessibility-critical pieces (menus, dialogs, popovers) over hand-rolled equivalents.

### 8.4 Adding a fixture

1. Export bookmarks from the target browser.
2. Run `scripts/anonymize-fixture.ts` to replace personal data with seeded fake data.
3. Commit under `fixtures/browsers/<browser>/<version>/`.
4. Add an entry to the round-trip test list in `crates/lantern-core/tests/parser_roundtrip.rs`.
5. Write a changelog entry under `Changed` or `Added` in the `fixtures` area.

## 9. Privacy considerations for contributors

Lantern has stronger-than-usual privacy commitments. A few practices help you stay aligned with them without thinking:

- **Before adding a dependency**, ask: does this crate or package make network calls? If you are not sure, check its `Cargo.toml` or `package.json`: look for `reqwest`, `hyper`, `tokio::net`, `isahc`, `ureq`, `axios`, `fetch` polyfills. If yes, it probably belongs (if anywhere) only in `lantern-net`.
- **Before adding a log line**, ask: does my format string contain a URL, a title, an email, a username, or any variable sourced from user data? If yes, do not add it. Log the *kind* of operation and the *count*, not the content.
- **Before adding a file write**, ask: is this path under the user's chosen output directory or the settings directory? If neither, stop and ask in the PR.
- **Before adding an auto-update check, a "call home" URL, a crash reporter, or an analytics library**, do not. These are not welcome in Lantern and CI will reject the PR.

None of this is meant to make contributing scary. It is meant to keep you out of a reviewer's "required:" bucket.

## 10. Troubleshooting

### 10.1 `cargo tauri dev` fails with a linker error

Usually a missing MSVC toolchain. Reinstall the VS Build Tools with the C++ workload selected and run `rustup show` to confirm your active toolchain is `stable-x86_64-pc-windows-msvc`.

### 10.2 WebView2 not found

Install the Evergreen Bootstrapper from Microsoft: https://developer.microsoft.com/microsoft-edge/webview2/.

### 10.3 Tests fail on Windows with path-length errors

Enable long paths in Windows: run `git config --global core.longpaths true` and enable the registry setting `HKLM\SYSTEM\CurrentControlSet\Control\FileSystem\LongPathsEnabled = 1` (requires a restart).

### 10.4 `cargo clippy` passes locally but fails in CI

Your local clippy version is different from CI's. Run `rustup update` and retry.

### 10.5 UI types don't match the Rust backend

`ts-rs` regenerates types on demand. If a type seems stale, run:

```
cargo test -p lantern-app export_bindings
```

This writes the updated `.ts` files into `ui/src/ipc/bindings/`. Commit those alongside your Rust struct changes.

### 10.6 Something else

Open a discussion thread or ask in the issue tracker. We'd rather answer a question than have you spend an hour stuck on environment setup.

## 11. Getting help

- **Issues:** bugs, feature ideas, documentation problems.
- **Discussions:** questions, design conversations, "is this a good idea?" chats.
- **PR comments:** specific to a change in flight.
- **Security reports:** see `SECURITY.md`, not the issue tracker.

There is no chat room by design. Important conversations happen where they can be found later.

## 12. A note to first-time contributors

It is perfectly fine if your first PR is a typo fix in this very document. A first PR's purpose is to shake out your local setup and get you comfortable with the process; its purpose is not to impress anyone. Small, clean, boring PRs are the backbone of a healthy project, and we are grateful for every one of them.

Welcome aboard.
