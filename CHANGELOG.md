# Changelog

All notable changes to Lantern are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Lantern uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

The pre-1.0 development cycle.  Lantern is at **0.1.0**; the road to a
future stable v1.0 continues, and breaking changes are still allowed
until then.  v1.0 (future) will be **the stability promise**, at which
point breaking changes will require a major-version bump.  Future
slices (CLI subcommand expansion, more locales, ARM64 build, MSIX
packaging hardening, etc.) continue on the way to v1.0.

### Fixed

- **ci:** the `changes` job no longer calls the Pulls API (which 403s
  under this account's restricted `GITHUB_TOKEN` — contents/metadata/
  packages only). Path filtering is now a `git diff`, and the workflow
  declares `permissions: contents: read, pull-requests: read`.
- **ci:** `reproducible-build-check` rebuilds twice from the same tree
  with `--remap-path-prefix` and MSVC `/Brepro`, instead of two sibling
  checkouts whose absolute paths made the hashes impossible to match.

---

## [0.1.0] - 2026-05-05

The first **0.1.0** release: an early, pre-1.0 milestone.  Ten months
of development across nine milestones (v0.0.1 through v0.0.11) converge
here.  Every PRD requirement either meets its NFR target or has a
documented operational caveat
(`private/docs/17-known-limitations-v0.1.md`).  357 Rust + 70 Vitest +
8 Playwright E2E specs, all green.  Code-signing scaffolding ships
unsigned pending certificate procurement; NVDA test plan ships ready
for first execution.

### Added

**Headless companion: `lantern-cli` crate**
- New `crates/lantern-cli/` workspace member shipping a `lantern`
  binary with three subcommands:
  - `lantern sanitize <INPUT> [-o OUTPUT] [--rule-set NAME |
    --rule-set-file PATH] [--dry-run]`: apply a built-in or
    file-loaded rule set to a Netscape bookmark file; defaults to
    `Minimal clean`; `--dry-run` prints the change summary without
    writing.
  - `lantern info <INPUT>`: print bookmark / folder / separator
    counts and max depth.
  - `lantern rule-sets [--list-builtin]`: list built-in rule-set
    names with one-line descriptions and treatment counts.
- Reuses `lantern-core` and `lantern-io` directly, proving the
  headless path that the Tauri GUI exercises.
- 5 integration tests via `assert_cmd` (round-trip parse +
  sanitize + emit) plus 2 unit tests.

**End-to-end test framework**
- New `ui/e2e/` directory with Playwright scaffolding
  (`@playwright/test ^1.59.1`) and 5 spec files covering 7 PRD user
  stories: US-001 (open file), US-002 (browse), US-003 (search),
  US-008/US-009 (preview + apply), US-013 (theme switch), US-018
  (keyboard accessibility).
- Tests drive the UI against the Vite dev server with browser-stubs
  mocking the Tauri IPC layer (opt-in via
  `localStorage["lantern.test.fixture"] = "small"`).
- New `ui-e2e` CI job: runs `playwright install chromium` then
  `npm run e2e`.  Marked `continue-on-error: true` until reliably
  green (same posture as `reproducible-build-check`).
- E2E coverage of the remaining 13 user stories
  (US-004..007/010..012/014..017/019..020) is a v1.x follow-up; the
  framework + first-pass coverage is the 0.1.0 deliverable.

**Repo-level release artefacts**
- `LICENSE-APACHE`: canonical Apache 2.0 text with the "How to apply"
  appendix filled in for "2026 The Lantern Authors".
- `LICENSE-MIT`: canonical MIT text, year 2026.
- `LICENSE`: 14-line top-level dual-licence pointer with
  `SPDX-License-Identifier: Apache-2.0 OR MIT`.
- `AUTHORS`: contributor roster (project-lead placeholder flagged
  with a TODO).
- `README.md` polished: tagline, status badges (CI + licence),
  "What is", "Why", "Install", "Use", "CLI", "Licence", "Security",
  "Contributing" sections.  101 lines.

**Release process docs**
- `private/docs/15-public-repo-flip-checklist.md`: maintainer's
  pre-flip checklist (secrets review, placeholder resolution, CI on
  cold clone, NVDA + cert prerequisites, version-tag creation,
  release-artefact attachment, repo settings).
- `private/docs/16-nvda-test-plan.md`, 10-test manual screen-reader
  test plan for NVDA-on-Windows: startup, file open, tablist,
  tree, list, modal open/close, form, toast, empty states.  Findings
  template references `private/audits/A11Y_NVDA_v0.1.0.md`.
- `private/docs/17-known-limitations-v0.1.md` (new): what's
  intentionally out of 0.1.0 and where it lives on the roadmap.

### Fixed

- **Signing-design correction.**  v0.0.10 routed signing through Tauri's
  `bundle.windows.signCommand` config field with a `${LANTERN_SIGN_CMD}`
  env-var indirection.  In practice Tauri 2 errors when `signCommand`
  resolves to an empty string at build time, which broke unsigned dev
  builds outright.  v0.1.0 strips the `signCommand` block from
  `tauri.{installed,portable}.conf.json` entirely; the CI signing job
  now invokes `signtool sign` as a post-build step against each
  produced artefact (`lantern.exe`, NSIS `.exe`, MSI `.msi`).
  `LANTERN_SIGNED=1` is still threaded into the cargo build so
  `build.rs` can flip the `BuildInfo.signed` flag.  Runbook
  (`14-signing-runbook.md`) and CI workflow updated to match.
- **MSIX target deferred to a future release.**  v0.0.10 listed `msix`
  in `bundle.targets`; in practice MSIX packaging needs publisher-
  identity certificate work and Tauri 2's MSIX bundler integration
  that's still maturing.  v0.1.0 reverts to NSIS + MSI for the
  installed flavor (MSI as v0.0.9's stand-in remains the production
  installer); MSIX returns when Microsoft Store publishing is
  scheduled.

### Operational caveats (not code defects)

- **Releases ship unsigned** until an Authenticode certificate is
  procured.  Signing scaffolding has been in place since v0.0.10; the
  CI `sign-windows-installed` job activates the moment
  `LANTERN_CODESIGN_PFX_BASE64` lands in repo secrets.  Until then,
  `BuildInfo.signed` reports `false` and About shows `unsigned`.
  See `private/docs/14-signing-runbook.md`.
- **NVDA manual session not yet conducted.**  Test plan in
  `16-nvda-test-plan.md` is ready for first execution.  Findings
  will land in `private/audits/A11Y_NVDA_v0.1.0.md` and any P0
  outcomes block a follow-up patch release.
- **ARM64 build target** still pending GitHub-hosted ARM Windows
  runners reaching GA.  The build configs handle x86_64 only.

### Intended stable surfaces (locked in at the future v1.0)

These are the surfaces intended to commit to backward compatibility for
the v1 line once v1.0 lands:

- **IPC command surface** (37 `tauri::generate_handler!` entries):
  no breaking changes without a v2.x bump.
- **`lantern-cli` subcommand surface**: `sanitize`, `info`,
  `rule-sets`.  New subcommands and flags are additive.
- **TOML settings file** (`<settings_dir>/settings.toml`): fields
  may be added; existing fields are read-stable.
- **Rule-set TOML format** (`*.lantern-rules.toml`): additive only.
- **Netscape bookmark file format**: output stays compatible with
  Chromium / Firefox / Safari import paths.

### Cumulative test counts at v0.1.0

```
lantern-core:     225 unit + 5 proptest        (parsing, model, sanitize, merge, search)
lantern-io:        27 unit                     (file IO, settings, rulestore, logstore)
lantern-net:       15 unit                     (dead-link checker)
lantern-app:       78 unit                     (35 IPC commands, settings, merge, search)
lantern-cli:        7 (2 unit + 5 integration) (CLI subcommands)
                  ───
Workspace total:  357

UI (Vitest):       70 across 15 component files + 5 hook/state files
UI (Playwright):    8 specs across 5 files       (non-blocking)
```

### Process / docs

- All version files at `0.1.0` (`Cargo.toml`, `package.json`,
  `ui/package.json`, `tauri.conf.json`).
- `Cargo.lock` regenerated.
- `private/docs/STATUS.md`, `private/docs/08-version-roadmap.md`,
  `private/docs/04-engineering-process.md` updated with the v0.1.0
  entry.

---

## [0.0.11] - 2026-05-05

The **"QoL + UI/UX perfection"** milestone: no new features, every
slice removes a small friction.  Toast/notification system replaces
ad-hoc inline error paragraphs; skeleton loaders replace `Loading…`
text; empty states replace blank panes; the tab bar gains middle-click
close + a right-click context menu; F2 / double-click triggers inline
rename in tree and list rows.  No backend churn: UI-only milestone
with one IPC reuse (`rename_node`).  350 Rust + 70 Vitest tests, all
green.

### Added

**Toast / notification system**
- New `ui/src/components/Toast.tsx` + `useToast` hook backed by a small
  Zustand store at `ui/src/state/toasts.ts`.  Auto-dismiss timers:
  4 s for `info`, 6 s for `success`; `error` toasts persist until
  dismissed manually.
- Bottom-right stack with slide-in animation (new keyframe in
  `index.css`).  z-index 70 (above modals' 50, below focus rings).
- Errors render in a dedicated `aria-live="assertive"` region;
  info/success share `aria-live="polite"`.
- Replaces the v0.0.7 ad-hoc `<p className="text-danger">{error}</p>`
  pattern across SettingsModal save errors, settings-panes load errors
  (Logs / About / Keyboard), MergePickerModal failures, PreviewPanel
  apply errors, and DeadLinkModal *background* failures
  (post-delete refresh, clipboard, re-run).  The DeadLinkModal's
  primary inline error display stays; it's the modal's whole point.
- 3 Vitest tests cover render, auto-dismiss timing (`vi.useFakeTimers`),
  and error persistence.

**Skeleton loaders**
- New `ui/src/components/Skeleton.tsx`: `<Skeleton>` (configurable
  `w-/h-/rounded-`), `<SkeletonText lines={n}>`, `<SkeletonRow>`
  (icon + title + url shapes).  All use Tailwind's `animate-pulse`.
- Wired into TreePane (6 rows on initial mount), ListPane (10 rows),
  PreviewPanel (4 lines), LogsPane (6 rows), KeyboardPane (8 lines),
  AboutPane (6 lines).  Old `Loading…` text removed.
- 1 Vitest test asserts row count.

**Empty states**
- New `ui/src/components/EmptyState.tsx`: icon + title + optional
  description + optional action button.  Wrapping div has `aria-label`
  so screen readers announce the empty-state context.
- Inline SVG icons (folder / magnifying glass / check / document
  page) keep the bundle small and theme-aware.
- Wired into TreePane (no folders yet), ListPane (empty folder + no
  search results, differentiated), DeadLinkModal (pre-run "click
  Check links to start" + post-run all-green "all N links responded
  OK"), LogsPane.
- 2 Vitest tests cover render and action click.

**Tab-bar UX**
- Middle-click (`onAuxClick` button 1) on a tab closes it.
- Right-click (`onContextMenu`) opens a `<ContextMenu>` anchored at
  `clientX/clientY` with four items: *Close tab*, *Close other tabs*,
  *Close tabs to the right* (disabled when right-clicked tab is the
  rightmost), *Close all tabs*.
- New `ui/src/components/ContextMenu.tsx`: generic positioning-aware
  popover with full keyboard nav (ArrowUp/Down wrap, Home/End, Escape
  closes, Enter activates).  Reusable.  3 Vitest tests.
- 3 new TabBar Vitest tests: middle-click closes, right-click opens
  menu, "Close other tabs" closes everything except right-clicked.

**Inline rename**
- F2 on a focused tree node or list row swaps the label for an
  `<input>` pre-filled with the current name and auto-selected.
- Double-click on the *label* (TreePane) or *title cell* (ListPane)
  also triggers rename.  Chevron, icon column, and other interactive
  cells are unchanged.
- Enter commits via `ipc.renameNode(tabId, nodeId, newName)`; Escape
  cancels (original name preserved); blur commits.  Errors route
  through `useToast()`.
- Replaces the previous "open a modal to rename" flow for the common
  case.  2 new Vitest tests (TreePane F2 → input → Enter calls
  `renameNode`; ListPane double-click → input → Escape preserves
  title).

**i18n strings (15 added)**
- Toast: `toast.dismiss`.
- Empty states: `empty.tree`, `empty.list`, `empty.list.description`,
  `empty.search`, `empty.search.description`, `empty.deadlinks.preRun`,
  `empty.deadlinks.allGreen`, `empty.logs`.
- Loading: `loading.generic`.
- Tab context menu: `tabBar.context.{closeTab,closeOthers,closeToRight,
  closeAll}`.
- Rename: `rename.placeholder`.

### Fixed

- `ui/src/components/SettingsModal.test.tsx` and
  `settings-panes/AboutPane.test.tsx` mock `BuildInfo` shapes were
  missing `signed: false` (added in v0.0.10); the test files compiled
  against the old type and would have broken any future rebuild from
  scratch.  Side-fixed by the toast agent.

---

## [0.0.10] - 2026-05-05

The **"Code signing + MSIX"** milestone: scaffolding only.  v0.0.10
lands every piece of the signing pipeline that doesn't require an
Authenticode certificate at build time: a signed Tauri config that
picks up cert thumbprint + `signtool` command from environment
variables, a CI job that activates only when a `LANTERN_CODESIGN_PFX_BASE64`
secret is configured, and a `signed: bool` field on `BuildInfo` that
the About pane surfaces.  When a cert is dropped into repo secrets,
signed releases start flowing without any further code changes.
The `installed` Tauri config switches from MSI placeholder to true MSIX.

### Added

**Tauri signing scaffold + MSIX**
- `crates/lantern-app/tauri.installed.conf.json`: switched from MSI
  to **MSIX** as the installer format.  The bundle config now
  references `${LANTERN_SIGN_CMD}` and `${LANTERN_CERT_THUMBPRINT}`
  env vars; locally these resolve to empty strings and Tauri silently
  produces an unsigned bundle (the right behavior for dev).
- `crates/lantern-app/tauri.portable.conf.json`: same env-var hooks
  added so the standalone `lantern.exe` can be signed via the same CI
  job.

**`BuildInfo.signed` field**
- New `signed: bool` field on `BuildInfo` (Rust + ts-rs binding +
  hand-written `ui/src/ipc/types.ts`).  Driven by a
  `LANTERN_SIGNED=1` env var read by `crates/lantern-app/build.rs`
  and exposed to the binary via `option_env!("LANTERN_SIGNED")`.
- `AboutPane` shows `✓ signed` (accent color, `aria-label="Authenticode-
  signed build"`) for signed builds and `unsigned` (muted) otherwise.
- 1 new Rust test (`get_build_info_signed_field_defaults_false_in_dev`)
  + 2 new Vitest cases (signed / unsigned label rendering).

**CI: `sign-windows-installed` job**
- Conditional on the `LANTERN_CODESIGN_PFX_BASE64` repository secret
  being set; on a fork or a repo without the secret, emits a
  `::notice::` and exits zero.  External contributors can still run
  unsigned builds locally without setup.
- When the secret is present: decodes the PFX, imports it into
  `Cert:\CurrentUser\My`, captures the SHA-1 thumbprint, exports
  `LANTERN_SIGNED=1` / `LANTERN_CERT_THUMBPRINT` /
  `LANTERN_SIGN_CMD`, runs `tauri build --config
  crates/lantern-app/tauri.installed.conf.json`, and uploads the
  resulting signed `.exe` (NSIS) and `.msix` artifacts.

**Documentation**
- `SECURITY.md`: new "Verifying release authenticity" section
  documents `Get-AuthenticodeSignature` for downloaded binaries and
  flags the v0.0.10 caveat (signed pipeline in place, ships unsigned
  until a cert is acquired).
- `private/docs/14-signing-runbook.md`, internal runbook for
  maintainers: how to enable signed CI builds (drop a base64 PFX
  + password into repo secrets), how to sign locally for ad-hoc
  testing, troubleshooting (signtool not on PATH, thumbprint
  mismatches, MSIX `Publisher` mismatches), and when to flip the
  v0.0.10 caveat in `SECURITY.md`.

### Changed

- `tauri.installed.conf.json` `bundle.targets`: `["nsis", "msi"]` →
  `["nsis", "msix"]`.  MSI was a v0.0.9 stand-in for true MSIX
  packaging.

### Deferred (still cert-gated)

- **The certificate itself.**  Scaffolding is in place; releases
  ship unsigned until a cert is in repo secrets.  Track in
  `private/docs/STATUS.md` "Next thing to do".
- **ARM64 build target**: still pending GitHub-hosted ARM Windows
  runners reaching GA.

---

## [0.0.9] - 2026-05-05

The **"Localisation, distribution, and signing"** milestone, minus the
signing.  v0.0.9 stands up the i18n framework so future locales can land
cheaply, splits the build into three explicit Tauri configs (portable /
installed / offline), opens the CI pipeline for x64 portable builds and
reproducibility verification, adds `SECURITY.md`, and exposes an SBOM
link in About.  Code signing slipped to v0.0.10: needs a certificate;
ships as soon as one is in hand.  349 Rust + 54 Vitest tests, all green.

### Added

**Localisation infrastructure (NFR-L-1)**
- New `ui/src/i18n/` module: `I18nProvider` context, `useT()` hook with
  `{param}` substitution, `Translations` type, per-locale registry.
- English-only at ship.  The shell (`TitleBar`, `StatusBar`, `TabBar`)
  and `SettingsModal` rail labels migrated to `t(...)` calls as the
  reference pattern; deeper components stay on string literals for now
  and follow as their owners migrate.
- Unknown keys fall through to the key string itself, so missing
  translations are visible-but-harmless during a mid-flight migration.
- Adding a locale is a drop-in file in `ui/src/i18n/` plus a one-line
  registry widen.  See `ui/src/locales/README.md` for the contract.
- 3 new Vitest tests cover known-key lookup, unknown-key fallback, and
  parameter substitution.

**Distribution: three explicit Tauri configs**
- `crates/lantern-app/tauri.portable.conf.json`: `bundle.active=false`;
  produces a single portable `lantern.exe` with no installer.
- `crates/lantern-app/tauri.installed.conf.json`: `bundle.targets`
  includes `nsis` and `msi` (MSI substitutes for true MSIX in this
  slice; MSIX needs Microsoft Store certificate work and slips with
  signing).
- `crates/lantern-app/tauri.offline.conf.json`: `productName="Lantern
  (Offline)"`; non-bundled output for the `--no-default-features` flavor
  that strips `lantern-net` from the dependency graph.
- Use any flavor via
  `cargo tauri build --config crates/lantern-app/tauri.<flavor>.conf.json`.

**CI: `.github/workflows/ci.yml`**
- New `build-portable-x64` job: builds the portable flavor with the
  new config; doc-only PRs skipped via a `dorny/paths-filter@v3`-gated
  `changes` job (per-job `paths-ignore` isn't supported, so the gate
  lives on a small upstream check job).
- New `reproducible-build-check` job: checks out the repo twice into
  `build-a/` and `build-b/`, builds the portable flavor in each,
  SHA-256-hashes both `lantern.exe`s, and asserts equality.  Marked
  `continue-on-error: true` until reliably green; flips to blocking
  when track record permits.
- v0.0.6's `offline-build-symbol-check` job kept unchanged.
- ARM64 deferred until GitHub-hosted ARM Windows runners are GA;
  comment in the workflow notes the dependency.

**Repo-level docs**
- `SECURITY.md` at the root: supported versions (`0.1.x`),
  vulnerability-report channel (placeholder `security@lantern.dev`,
  replace once a project mailbox exists), 5-business-day acknowledgement
  target, 90-day coordinated-disclosure window, local-only privacy
  reminder.

**About pane**
- `AboutPane` gains an SBOM link in its definition list.  Target URL
  is a `releases/` placeholder (TODO until first signed release lands);
  the link itself is wired with `target="_blank"`,
  `rel="noopener noreferrer"`, and a focus-visible ring.
- 1 new Vitest case asserts the link's role / target / rel attributes.

### Deferred to v0.0.10 (cert-gated)

- **Code-signed binaries**: Authenticode for x64 + ARM64 portable +
  NSIS + MSIX.  Needs a certificate; ships when one is acquired.
- **True MSIX packaging**: needs Microsoft Store cert work.  v0.0.9
  ships MSI as the installed format in the meantime.
- **ARM64 build target**: runner-cost concern; flips on when
  GitHub-hosted ARM Windows runners are GA.

### Deferred to v1.0.0-rc1

- **E2E coverage** for the 20 PRD user stories (US-001..US-020).
- **NVDA manual test session**: screen-reader sweep with notes.
- **Release-readiness sweep**: final LICENSE decision, copyright
  headers, AUTHORS file, README polish, public-repo flip checklist.

### Long-tail carry-overs (no scheduled milestone)

- **Cross-tab drag-and-drop merge trigger (F-MULTI-3)**: UX-heavy;
  v0.0.7's modal picker covers the workflow.
- **Shortcut rebinding**: read-only listing shipped in v0.0.7; the
  rebind UI is a separate UX problem.

---

## [0.0.8] - 2026-05-05

The **"Performance hardening (rest) + A11Y P2"** milestone: closes
every v0.0.7 deferred item and retires the v0.0.6 audit completely.
Three slices: progressive tree expansion in `TreePane`, background
search-index baseline gating NFR-P-4, and the final A11Y polish round.
349 Rust + 50 Vitest tests, all green.

### Added

**Progressive tree expansion**
- `TreePane` rewritten to fetch only the top-level folders on mount via
  the new `get_tree_root` IPC command; expanding a folder fetches its
  immediate children via `get_tree_children` (cached per-tab).
  Complements v0.0.7's virtualised `ListPane`; large bookmark files
  (deep trees) now load instantly because nothing below the root level
  is fetched until the user asks for it.
- New `TreeNodeLazy` IPC type: lighter than the recursive `TreeNode`,
  carries a `has_children` hint so the UI knows whether to render an
  expand chevron without fetching the children.
- Existing `TreeView` / `TreeNode` / `get_tree` kept unchanged for
  backward compatibility.
- 6 new Rust tests for `get_tree_root` / `get_tree_children` + 2 new
  Vitest tests for `TreePane` (asserts only top-level rows are in the
  DOM after mount; expanding fetches and renders children).

**Background search indexing baseline**
- New `lantern_core::search::SearchIndex`: per-tab inverted index keyed
  on lowercased token prefixes (full token + 3- and 4-char prefixes)
  drawn from bookmark titles and URLs.  AND semantics across
  whitespace-separated query terms.
- The `search` command consults the index to narrow the candidate set
  before the matcher's verification pass.  Index is built lazily on
  first query, invalidated on document mutations
  (`apply_changeset` / `undo` / `redo` / `rename_node` / `delete_node` /
  `create_*` / `move_node`) and dropped on tab close.
- Regex mode skips the index (avoids pulling `regex_syntax` for the
  v0.0.8 baseline; documented in module rustdoc).
- Build performance: < 100 ms cold over 25 k bookmarks (gated as a
  release-profile assertion); query: < 1 ms.  Closes NFR-P-4 (regex
  search < 150 ms at 25 k).
- Per-tab storage in `AppState` via a parallel `RwLock<HashMap<TabId,
  Option<SearchIndex>>>` rather than restructuring `OpenTab`: keeps
  call sites unchanged.
- 10 unit tests in `SearchIndex` (8 functional + 2 perf-gated) + 2
  integration tests in `commands.rs` confirming results match
  with-and-without index and that mutations trigger rebuild.

### Changed

**A11Y P2 polish: closes the v0.0.6 audit completely**
- Color-contrast retuning on dark + light themes: every text/surface
  pair now ≥ 4.5:1 (WCAG AA).  Dark: `--danger` red-500 → red-400
  (3.8:1 → 4.7:1); `--border` neutral-800 → neutral-700 (3:1 → 4.1:1).
  Light: `--accent-hover` amber-600 → amber-700 (3.6:1 → 5.5:1);
  `--border` neutral-300 → neutral-400 (1.5:1 → 2.4:1).
- Focus-visible rings on the remaining controls: TitleBar window
  controls (3 buttons), Tools menu trigger, Settings gear; ListPane
  column-header sort buttons, filter-toggle chevron, clear-search,
  search-submit; FilterDrawer chip remove buttons; close buttons in
  DeadLinkModal / DiffModal / SettingsModal / RuleSetEditorModal (also
  given explicit hover backgrounds).
- Touch targets bumped to PRD NFR-A-5 minimums: TitleBar window
  controls `w-10` → `w-12` (40 → 48 px); Settings gear `w-8` → `w-10`;
  modal close buttons given explicit `w-8 h-8` hit areas; TabBar close
  button `min-w-[24px]` → `min-w-[32px]`.
- `private/audits/A11Y_AUDIT_v0.0.6.md` Status table now reads
  `✅ All closed in v0.0.8` for the P2 row; per-item entries kept
  verbatim for archival reference.

### Fixed

- `lantern-app::commands::tests::get_build_info_reports_workspace_version`
  asserted `"0.0.7"` literally; switched to `env!("CARGO_PKG_VERSION")`
  so the test follows future version bumps without manual edits.

### Process / docs

- `03-technical-design.md` §12 updated: moves "v0.0.6 planned" into
  "v0.0.7 current" (offline-only build flavor shipped in v0.0.6;
  reproducible-build CI now slated for v0.0.9).
- `04-engineering-process.md` header refreshed to v0.0.8.
- `08-version-roadmap.md`: v0.0.7 entry moved to "Shipped releases";
  v0.0.8 milestone slot opened, then closed.
- `13-strategy.md` Q1 row marks v0.0.6 + v0.0.7 shipped and D-1
  resolved.
- ADR-0009 status line updated to "Implemented in v0.0.7".

---

## [0.0.7] - 2026-05-05

The **"Merge & settings"** milestone.  v0.0.7 ships the long-deferred
cross-document merge feature (ADR-0009 ratified in v0.0.6, implementation
landed here), rounds out the settings UI with three new panes
(Keyboard / Logs / About), closes the v0.0.6 A11Y audit's P1 backlog
(Tools menu keyboard nav, breadcrumb landmark, live region on the offline
indicator, focus management in PreviewPanel, chip labels in FilterDrawer),
and lands the first slice of performance hardening (virtualised
`ListPane`).  330 Rust + 48 Vitest tests, all green.

### Added

**Cross-document merge: flagship feature**
- New `lantern_core::model::merge` module: pure-core merge planner with
  `MergePlan { picks, strategy, merged_root_name }` and
  `build_merged_document(sources, plan) -> Result<Document, MergeError>`.
  Conflict strategies: `KeepFirst` (later URL duplicates dropped),
  `KeepNewest` (most-recent `add_date` wins), `KeepBoth` (titles
  disambiguated with " (n)" suffix).  Same-name folders under the merged
  root are recursively unioned.  13 unit tests covering empty plans,
  out-of-range source indices, unknown node-ids, identity-modulo-ids
  cloning, deep-subtree id reassignment, every strategy, and an explicit
  "source documents are unmodified" assertion.
- New `merge_documents` IPC command in `lantern-app`: translates the
  UI's tab-id-based picks into core `MergePick`s, runs the planner, and
  opens the merged result as a new tab.  5 integration tests.
- New `MergePickerModal` UI: collapsible per-tab folder picker, picks
  list with chip removal, three-radio strategy selector with tooltips,
  root-name input.  Visual style mirrors `DiffModal`.  Wired into
  `useFocusTrap` for Tab/Shift+Tab cycling and Escape-to-close.
- Tools menu → "Merge documents…" entry in `TitleBar`.  Keyboard
  reachable; obeys the menu's existing arrow-key / Home / End nav.
- Implements ADR-0009 in full: merged document carries a fresh `NodeId`
  allocator and empty undo/redo stacks; source documents stay read-only.
  No changes to `NodeId`, `Document::apply/undo/redo`, or any existing
  command surface.

**Settings UI completeness: left-rail layout**
- `SettingsModal` rewritten with a four-pane left-rail layout:
  General / Keyboard / Logs / About.  Roving `tabIndex`, Arrow-Up /
  Arrow-Down wraparound, Home / End jumps, `role="tablist"` /
  `role="tab"` / `role="tabpanel"` semantics.  General pane retains the
  v0.0.6 theme picker, list-density toggle, and dead-link checker
  opt-in; only the General pane mutates settings, so Save is wired
  solely to that path.
- New `KeyboardPane`: read-only reference table grouped by category
  (File / Edit / View / Tools / App), backed by the `list_shortcuts`
  IPC command which returns the canonical PRD §8.9 list.  Footer note
  signals that rebinding ships in a future release.
- New `LogsPane`: pulls the last 200 lines from
  `<settings_dir>/logs/lantern.log` via `get_logs`.  Severity colouring
  (info / warn / error), Refresh button, empty-state message.
- New `AboutPane`, definition-list of build metadata via
  `get_build_info`: version, build flavor (`default` vs `offline-only`),
  Rust version, license, ADR index path.
- Three new IPC commands in `lantern-app`: `list_shortcuts`, `get_logs`,
  `get_build_info`.
- New `lantern_io::logstore` module: owns log-path resolution (mirrors
  the settings-path precedence: env override → portable mode →
  `%APPDATA%`/XDG/macOS) and a small "tail-the-file" parser with bounded
  memory via a ring buffer.  5 unit tests.

**A11Y P1 cleanup: audit backlog closed**
- Tools menu (`TitleBar`) now supports full keyboard nav: ArrowUp /
  ArrowDown wrap, Home / End jump.  Audit §P1 #16.
- `PreviewPanel` manages focus: opening focuses the first heading;
  closing restores focus to the trigger row.  Audit §P1 #14.
- `Breadcrumb` exposed as a navigational landmark
  (`<nav aria-label="Breadcrumb">`) so screen readers announce it
  separately from the list.  Audit §P1 #10.
- OFFLINE indicator in the status bar is now a `role="status"` live
  region with `aria-live="polite"`, so the network-state flip is
  announced without stealing focus.  Audit §P1 #17.
- `FilterDrawer` chip inputs received accessible labels and the
  drawer's expand/collapse chevron declares `aria-expanded` /
  `aria-controls`.  Audit §P1 #7, #8, #9.

**Performance hardening: first slice**
- `ListPane` is now virtualised via `react-window`'s `FixedSizeList`.
  Bookmark lists with 50 k+ rows render only the visible window
  (typically 20-40 rows) plus a small overscan, dropping initial-paint
  cost from O(n) to O(viewport).  Row height tracks the existing
  `data-density` attribute (28 px compact / 36 px comfortable) via a
  `MutationObserver`.  Keyboard navigation calls `scrollToItem` so the
  focused row stays in view when arrow-keying past the viewport.
  New `useElementSize` hook wraps `ResizeObserver` for the parent
  container.  Addresses NFR-P-1 and NFR-P-3.
- New dependency: `react-window ^1.8.11` (+ `@types/react-window`,
  devDep).  No transitive network code.

**Process / docs**
- v0.0.7 development on Rust 1.95 surfaced three new clippy lints
  (`collapsible_match`, `useless_conversion`, `writeln_empty_string`)
  on previously-clean code.  All resolved in-place; `collapsible_match`
  in `sanitize/apply.rs::write_flag` opted for a localised
  `#[allow(...)]` because the obvious refactor (pattern guard) is
  blocked by E0596 (guards bind immutably, but the recursion needs
  `&mut Folder`).

### Fixed

- `lantern-app` `commands.rs`: `bookmarks.into_iter().zip(results.into_iter())`
  flagged by clippy 1.95's `useless_conversion`; second `into_iter()`
  removed (no behavior change).
- `lantern-io` `logstore.rs` test code used `writeln!(f, "")` to emit a
  blank line; clippy 1.95's `writeln_empty_string` suggests
  `writeln!(f)` (same output).

### Deferred to v0.0.8

- Progressive tree expansion (`TreePane` lazy-loads children on expand):
  second slice of performance hardening; complements v0.0.7's
  virtualised `ListPane`.
- Background search indexing baseline: gates NFR-P-4 (regex search
  < 150 ms at 25 k bookmarks).  Will live in `lantern_core::search`
  alongside the existing matcher and be held in `AppState` per tab.
- A11Y P2 polish from the v0.0.6 audit (color-contrast retuning beyond
  the v0.0.6 light-theme tokens, focus-visible rings on the remaining
  controls, additional touch-target work).
- Shortcut rebinding: v0.0.7 lists shortcuts read-only; rebinding is
  a separate UX problem.

### Deferred to v0.0.9

- Localization infrastructure (English-only at ship; the framework was
  not wired in this milestone).
- MSIX bundle target and ARM64 build target.
- Code-signed binaries and reproducible-build verification.

---

## [0.0.6] - 2026-05-05

The "accessibility, theming, and offline build" milestone.  Lantern picks up
its first round of structural a11y work, a long-promised light theme, and a
build flavor that proves no networking stack is linked when the user opts
out of the dead-link checker.  The **D-1** decision (cross-document-merge
`NodeId` scope) is captured in ADR-0009, unblocking v0.0.7 merge work.
295 Rust + 34 Vitest tests, all green.

### Added

**A11Y: keyboard + screen-reader foundations**
- `useFocusTrap` hook (`ui/src/hooks/useFocusTrap.ts`): Tab/Shift+Tab cycle,
  Escape-to-close, focus-restore on unmount, `data-autofocus-skip` opt-out.
  3 Vitest cases covering wraparound, Escape, and focus restoration.
- Focus traps wired into all four modals: `DeadLinkModal`,
  `RuleSetEditorModal`, `DiffModal`, `SettingsModal`.  Tab no longer escapes
  the dialog; Escape closes (respecting the dirty-confirm guard in the rule
  editor).
- `TabBar` rewritten with proper tablist semantics: `role="tablist"`,
  per-tab `role="tab"` + `aria-selected` + `aria-controls`, ArrowLeft /
  ArrowRight wraparound, Home / End, roving `tabIndex`.  Close button hit
  area increased to 24×24.
- `TreePane` expand/collapse buttons declare `aria-expanded` and
  `aria-controls`, sit in a 24×24 hit area, and surface a focus ring.
- App workspace `<main>` exposes `role="tabpanel"` + `aria-labelledby` so
  the TabBar's `aria-controls` resolves to a real element.

**Theming: light theme + system + reduced motion**
- Light-mode design tokens at `:root[data-theme="light"]` in
  `ui/src/index.css`.  Surfaces flip to off-white; accent stays amber for
  brand consistency; danger / diff / border / scrollbar tokens retuned for
  ≥ 4.5:1 contrast on white.
- `useTheme` hook (`ui/src/hooks/useTheme.ts`): resolves the persisted
  `ThemeSetting` to dark/light, applies `data-theme` on `<html>`, and
  subscribes to `prefers-color-scheme` flips when the user is on "System".
- Theme picker in `SettingsModal` (System / Light / Dark) with three-way
  radio bound to `getSettings`/`updateSettings`.
- `prefers-reduced-motion: reduce` media query disables `.animate-fade-in`
  and `.animate-pulse-dot` for users with vestibular sensitivities.
- `ThemeSetting` enum now ships System / Light / Dark variants on both
  sides of the IPC boundary (was Dark-only in v0.0.5).

**Distribution: offline-only build flavor**
- `lantern-app` Cargo features: `default = ["checker"]`,
  `checker = ["lantern-net/checker"]`.  Building with
  `--no-default-features` produces a binary with no `reqwest` and no
  `hyper` in the dependency graph.
- New CI job `offline-build-symbol-check` (`.github/workflows/ci.yml`) that
  runs `cargo build --release -p lantern-app --no-default-features` and
  asserts `cargo tree` contains neither `reqwest` nor `hyper`.  `tokio` is
  intentionally not asserted; Tauri 2 itself depends on it for its plugin
  runtime, so checking it would fail unconditionally.

**Decisions**
- ADR-0009 (`private/adrs/ADR-0009-cross-document-merge-nodeid-scope.md`)
  ratifies the provisional D-1 resolution: a merged document is a fresh
  `Document` with its own `NodeId` allocator; subtrees from the source
  documents are deep-cloned with reassigned ids; the merged document has
  empty undo/redo stacks and source documents stay read-only.  Concretely
  this means `NodeId`, `Document`, `UndoEntry`, `InverseKind`, and
  `Document::apply/undo/redo` need **no** changes in v0.0.7: only a new
  `model::merge` module and a single `merge_documents` IPC command.

**Process / docs**
- `private/audits/A11Y_AUDIT_v0.0.6.md`: 25 prioritised findings (P0/P1/P2)
  covering focus management, keyboard nav, ARIA semantics, touch targets,
  and color contrast.  P0 items shipped in this release; P1/P2 carry into
  v0.0.7.

### Fixed

- **ts-rs `export_to` path bug** (`crates/lantern-app/src/types.rs`): the
  v0.0.5 redesign moved the crate from `src-tauri/` to
  `crates/lantern-app/`, but the binding-export path was never updated.  All
  30 `#[ts(export_to = "../../ui/src/ipc/bindings/")]` annotations now use
  `"../../../ui/src/ipc/bindings/"` so regen lands in the actual UI tree
  rather than a phantom `crates/ui/` directory.  The phantom directory has
  been removed.  No runtime behavior change; the previously consumed
  `ui/src/ipc/bindings/` happened to remain in sync with current types.

### Resolved decisions

- **D-1**: Cross-document-merge `NodeId` scope.  Provisional resolution
  in v0.0.5 is now ratified via ADR-0009.  Implementation lands in v0.0.7.

### Deferred to v0.0.7

- A11Y P1 items (Tools menu keyboard, ChipInput labels, breadcrumb
  landmarks, live regions on the offline indicator); see audit §P1.
- Cross-document merge implementation (decision ratified, code not yet
  written).
- Full settings UI (Keyboard / Logs / About panes; shortcut rebinding).
- Performance hardening (virtualised list, progressive tree, background
  search indexing).

### Deferred to v0.0.9

- Localization infrastructure (English-only at ship; the framework was not
  wired in this milestone).
- MSIX bundle target and ARM64 build target.
- Code-signed binaries and reproducible-build verification.
- A11Y P2 polish (color contrast retuning, additional touch-target work).

---

## [0.0.5] - 2026-05-02

The "structured browse" milestone.  Browse and search gain a unified
**six-axis filter drawer** (kind / added-date / domain / TLD / scheme /
folder-depth) backed by a server-side `FilterSpec`, and the dead-link checker
graduates from "list" to "workbench" with clickable status filters, sortable
columns, multi-select, **bulk delete**, and clipboard export.  A new
**partial export** lets you write any folder out as its own self-contained
bookmark file.  295 Rust + 30 Vitest tests, all green.

The codebase was also redesigned out-of-band before v0.0.5 dev: flattened
project layout, npm workspaces with one root lockfile, a unified
`npm run {dev,build,test,lint,check,fmt,bindings}` command surface,
`src-tauri/` renamed to `crates/lantern-app/`, and a public/private split
that keeps internal docs out of the repo.

### Added

**Core / App**
- `FilterSpec` IPC type: six independent allowlist axes that compose with
  AND.  Threaded through both `get_folder_items` (browse) and `search`.
- `ExportScope::Subtree { root_id }`: export a single folder as a
  self-contained Netscape bookmark file.  Source-document undo state is
  not propagated; the source path is preserved so the F-EXP-7 overwrite
  guard still fires.
- `ItemKind` is now `Deserialize + PartialEq + Eq` so the kinds axis of
  `FilterSpec` can be compared at runtime.

**UI**
- `FilterDrawer` component (`ui/src/components/FilterDrawer.tsx`): six
  sections (Kind / Added / Domain / TLD / Scheme / Depth), chip inputs for
  domain & TLD, native date pickers for Added, depth bracket visible only
  during search.  State lives in the Zustand store and is preserved across
  navigation, sort changes, and pagination.
- Dead-link modal:
  - Status summary cards toggle as filters (click "4xx" → table shows only
    those entries).
  - Sortable columns with chevron indicators (Status / Time / Bookmark).
  - Per-row checkbox + "select all visible" header checkbox.
  - **Bulk delete** with a confirmation strip; each delete is independently
    undoable from the source document.
  - "Copy URLs" copies the visible-filtered-sorted entries to the
    clipboard.
  - Loading skeleton while the initial check is running.
- Detail pane gains an **export-folder** action (download icon) that writes
  the selected folder to a user-chosen path via the `subtree` scope.

**Build / dev**
- `[profile.dev]` keeps our crates at `opt-level = 0` for debuggability;
  `[profile.dev.package."*"]` pegs dependencies at `opt-level = 3` so
  cargo-test runs aren't molasses.
- `[profile.release]` now uses `lto = "thin"`, `codegen-units = 1`,
  `strip = "symbols"` for a smaller, faster binary.
- Single-binary rename: `[[bin]] name = "lantern"` so the workspace
  produces `target/release/lantern.exe` (was `lantern-app.exe`).

### Changed

- `ExportScope` is now a tagged union: `{ kind: "whole_document" }` or
  `{ kind: "subtree", root_id }`.  Previous JS callers passing the bare
  string `"whole_document"` need to wrap.
- `lantern-ui` is an npm workspace member; root `npm install` resolves
  everything.  No more dual-lockfile install.
- All dev commands run from the repo root via `npm run *`.  CI mirrors
  the same surface.
- CI `cargo` jobs run the full workspace (clippy, test, fmt) and Vitest
  is now exercised on every pull request.

### Fixed

- Tauri release artifact path corrected in CI from
  `src-tauri/target/release/lantern.exe` to the workspace
  `target/release/lantern.exe`.

### Resolved decisions

- **D-5**: `lantern-app` crate directory renamed `src-tauri/` →
  `crates/lantern-app/`.  Tauri CLI is invoked with
  `--config crates/lantern-app/tauri.conf.json` from the repo root.
- **D-6**: `bundle.active` flipped to `true` so release builds produce
  the NSIS installer.

### Deferred

- **D-1 (cross-document merge)**: `NodeId` scope after merge needs more
  design work; deferred to v0.0.6.  Decision (provisional, will revisit):
  the merged document is its own coherent entity with a fresh `NodeId`
  space; source documents stay read-only inputs to the merge operation
  and undo entries reference only the merged copy.

---

## [0.0.4] - 2026-04-25

The "v1 of polish" milestone: a new `lantern-net` crate adds an opt-in
dead-link checker, the rule-set editor finally exposes per-treatment config
inputs (custom QP list, regex pattern + replacement), a document-level diff
view lets users compare two open tabs, and a comfortable list-density toggle
joins the settings panel.  Backed by a Vitest suite for the preview-panel
chain and 282 passing Rust tests across the workspace.

### Added

**Core**
- `lantern_core::diff` module: URL-keyed three-way diff between two
  `Document`s (`added` / `removed` / `modified` buckets, each carrying a
  `BookmarkSnapshot` view).  O(N + M) implementation backed by 8 unit tests.
- `Treatment::current_config()`: round-trips per-treatment configuration
  back to TOML so rule-set persistence captures live state for parameterised
  treatments.
- Configurable treatments now serialise their state: `url.qp.custom` writes
  its `params` array, `title.regex` / `folder.regex` write `pattern` +
  `replacement`.

**Net (new crate)**
- `lantern-net`: feature-gated dead-link checker (`reqwest` + `rustls`,
  `tokio` runtime).  Stable `LinkStatus` IPC enum (`ok` / `redirect` /
  `client_error` / `server_error` / `timeout` / `network_error` / `skipped`)
  plus `should_check` / `classify_status` pure helpers always available
  regardless of feature flags.  15 unit tests; the `checker` feature is
  enabled in the Tauri app.

**IO**
- `rulestore::save_rule_set_with_configs`: companion to `save_rule_set` that
  takes `(treatment_id, Option<toml::Value>)` pairs so the editor can persist
  parameterised treatments without losing state.
- `Settings::list_density`: new `compact` / `comfortable` enum, defaults to
  `compact` for backwards compatibility.

**App / IPC**
- 2 new Tauri commands (29 → 31 registered):
  - `save_rule_set_with_configs`: persists rule sets with per-treatment
    configuration blocks.
  - `compare_tabs`: returns a `DocDiffReport` between two open tabs.
  - `check_dead_links`: probes every bookmark in a tab; gated by the
    `dead_link_checker_opt_in` setting (PRD NFR-PRIV-2).
- New IPC types: `RuleSetTreatment`, `LinkCheckEntry`, `LinkCheckReport`,
  `LinkStatusView`, `SkipReasonView`, `BookmarkSnapshotView`,
  `ModifiedBookmarkView`, `DocDiffReport`, `ListDensitySetting`.
- Recursive TOML ↔ JSON converters at the IPC boundary so per-treatment
  config blocks round-trip cleanly through ts-rs bindings.

**UI**
- Rule-set editor now renders inline config editors below each parameterised
  treatment row: chip-style param list for `url.qp.custom`, pattern +
  replacement inputs for `title.regex` / `folder.regex` with live regex
  validation.
- `DiffModal` (Ctrl + Shift + D): pick two tabs and see added / removed /
  modified bookmarks in a three-bucket layout with diff-coloured rows.
- Comfortable density toggle in Settings → "List density" (~36 px rows
  instead of the historical ~28 px).
- Dead-link checker setting is now active (no longer a stub) and feeds the
  privacy gate on the new `check_dead_links` command.
- Vitest test infrastructure: `vitest.config.ts`, jsdom env, browser-stubs
  for `@tauri-apps/*`, and 18 tests covering `DiffLine`, `ChangeRow`,
  `PreviewPanel`.

### Changed

- Workspace version bumped to `0.0.4`; `lantern-net` added as a workspace
  member.
- `RuleSetDetail` now exposes a parallel `treatments: Vec<RuleSetTreatment>`
  alongside `treatment_ids`, pairing each treatment with its current JSON
  config (`null` for stateless treatments).
- `DetailPane` extracted: `DiffLine`, `ChangeRow`, `PreviewPanel` moved to
  their own files in `ui/src/components/` so the Vitest suite can target
  them in isolation.

### Notes

- The structured filter drawer (date range / domain / TLD / scheme / depth)
  remains deferred to v0.0.5.
- ARM64 build target stays deferred indefinitely.

---

## [0.0.2] - 2026-04-19

Rule-set management out of `commands.rs` and onto disk, char-level diff in the
preview panel, settings persisted to `%APPDATA%\Lantern`, and a foundation of
property and fuzz tests so later refactors can move fast without regressions.

### Added

**Core**
- `sanitize::diff` module: LCS-based char-level diff (`DiffTag::{Equal,Removed,Added}`,
  `DiffSpan`, `char_diff`) with a 4096-char input guard. Feeds the preview panel's
  strikethrough/highlight rendering.
- Proptest suite (`crates/lantern-core/tests/proptest_roundtrip.rs`), 5 properties:
  `parse(emit(doc)) == doc` structurally, UTM strip / whitespace / HTML entities
  idempotence, and diff-span reassembly.
- `cargo-fuzz` scaffold (`crates/lantern-core/fuzz/`): two targets (`fuzz_parser`,
  `fuzz_url_treatments`), separate crate so nightly sanitizers don't pollute the
  main workspace.

**IO**
- `lantern_io::rulestore`: disk-backed rule-set store writing `*.lantern-rules.toml`
  files. Seeds the 3 built-ins (Minimal clean / Aggressive scrub / Full scrub) on
  first launch; `list / load / save / delete / duplicate` operations with graceful
  fallback to the built-in catalogue when an on-disk file is missing.
- Platform-aware settings directory resolution: `LANTERN_SETTINGS_DIR` env override,
  portable mode (settings.toml next to the exe), then `%APPDATA%\Lantern` on
  Windows / `$XDG_CONFIG_HOME/lantern` on Linux / `Application Support/Lantern`
  on macOS.

**App / IPC**
- 8 new Tauri commands: `get_settings`, `update_settings`, `list_rule_sets`,
  `get_rule_set`, `save_rule_set`, `delete_rule_set`, `duplicate_rule_set`,
  `list_treatments`.
- `ChangeEntry` extended with `before_spans` / `after_spans` for char-level diff
  rendering. Backwards-compatible; empty arrays when diff is skipped.
- `run_pass` now loads rule sets from disk (`rulestore::load_rule_set`) instead
  of the hardcoded catalogue, letting the UI point at user-created rule sets.

**UI**
- `DiffLine` component in `DetailPane` renders per-span strikethrough-red
  (removed) / highlighted-green (added) text with graceful fallback to the
  legacy line-through style when spans are empty.
- New Tailwind colors `diff-removed` / `diff-added` bound to CSS variables
  (`--diff-removed` `rgb(248 113 113)`, `--diff-added` `rgb(74 222 128)`).
- IPC surface expanded: `getSettings`, `updateSettings`, `listRuleSets`,
  `getRuleSet`, `saveRuleSet`, `deleteRuleSet`, `duplicateRuleSet`,
  `listTreatments`.
- New TS types: `AppSettings`, `ThemeSetting`, `RuleSetSummary`, `RuleSetDetail`,
  `TreatmentInfo`, `DiffSpan`, `DiffTag`, all regenerated via ts-rs from Rust.

### Changed

- Workspace `exclude = ["crates/lantern-core/fuzz"]` so the fuzz crate stays out
  of `cargo build` / `cargo test` on stable toolchains.
- `AppState` gains a `rules_dir` field alongside the existing `settings_path`.

### Fixed

- Several clippy lints under Rust 1.94's stricter rule set: `derivable_impls` on
  `EmitOptions::default`, `unnecessary_map_or` → `is_some_and`, `len_zero` →
  `!is_empty()`, `needless_lifetimes` on `find_folder`, `ptr_arg` on
  `sort_items`, `type_complexity` factored into a `SearchMatcher` alias,
  `bool_assert_comparison` in settings tests.

### Deferred to v0.0.3

- **Cross-field treatments** (`cross.dedupe`, `cross.empty_folders`): require
  extending the `Change` model with a `DeleteNode` variant.
- **`url.host.unshorten.offline` Change emission**: detection ships in v0.0.2,
  flag mutation needs the same model extension.
- **Per-treatment config** in rule-set TOML (`url.qp.custom`, `title.regex`,
  `folder.regex`): schema is stable, UI surface is not.
- **Rule-set editor UI**: backend ready, UI deferred.
- **Settings modal UI**: backend ready, UI deferred.

See [`docs/v0.0.3-roadmap.md`](docs/v0.0.3-roadmap.md) for the full plan.

### Privacy

No network requests added in v0.0.2. The opt-in dead-link checker remains
unimplemented and off by default.

---

## [0.0.1] - 2026-04-17

First working release. Opens a Netscape bookmark file, explores it in a three-pane layout, runs basic sanitization passes, previews and applies changes, and exports a clean copy.

### Added

**Core**
- Netscape bookmark HTML parser (`html5ever`) producing a typed `Document` tree
- `lantern-core` crate: `Node`, `Folder`, `Bookmark`, `Separator`, `Document`, `HeaderMetadata`, `DocumentStats` data model
- `lantern-io` crate: bookmark file read/write, TOML settings, rule set persistence
- Sanitization engine: `Treatment` trait, `ChangeSet`, `ApplyReport`, pass executor, single-level undo/redo
- Built-in treatments: `url.qp.utm`, `url.qp.click_ids`, `url.qp.session`, `title.whitespace`
- Built-in rule sets: "Minimal clean" (UTM + title whitespace) and "Aggressive scrub" (all tracking params + title whitespace)

**App shell**
- Tauri 2 app shell (`lantern-app` crate) with typed IPC commands via `ts-rs` bindings
- Frameless window with custom title bar, tab bar, and status bar
- Zero network requests by default; network indicator in status bar

**UI**
- Three-pane layout: folder tree (TreePane), item list (ListPane), detail/sanitize panel (DetailPane)
- Search: title and URL search with scope checkboxes, results banner, clear
- Column sorting in list view (title, URL, date added)
- Rule set picker with "Minimal clean" / "Aggressive scrub" selector and live description
- Change set preview panel with per-change approval checkboxes and All/None quick-select
- Full-document export via native save dialog

**Infrastructure**
- Root `package.json` with `@tauri-apps/cli ^2`; `npm run dev` / `npm run build` from repo root
- GitHub Actions CI: Rust (test + clippy + fmt), TypeScript (tsc --noEmit), Build (Tauri release + artifact upload)
- Application icon: hand-crafted SVG lantern (gradient glass body, flame, bail arch); all platform sizes generated via `tauri icon`
- Portable build (zip + `.exe`) produced by `npm run build`

### Privacy

No network requests are made in v0.0.1. The dead-link checker (opt-in, off by default) is not yet implemented.

---

[Unreleased]: https://github.com/coatyl/lantern/compare/v0.0.6...HEAD
[0.0.6]: https://github.com/coatyl/lantern/releases/tag/v0.0.6
[0.0.5]: https://github.com/coatyl/lantern/releases/tag/v0.0.5
[0.0.4]: https://github.com/coatyl/lantern/releases/tag/v0.0.4
[0.0.3]: https://github.com/coatyl/lantern/releases/tag/v0.0.3
[0.0.2]: https://github.com/coatyl/lantern/releases/tag/v0.0.2
[0.0.1]: https://github.com/coatyl/lantern/releases/tag/v0.0.1
