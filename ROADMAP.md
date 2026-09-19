# Lantern Roadmap

Lantern today is a focused, local-only bookmark-hygiene tool: it opens a
`bookmarks.html`, lets you browse and scrub it through a reviewable diff, and
writes a clean copy — with a Rust core (`lantern-core`), a strict I/O boundary
(`lantern-io`), a single feature-gated network crate (`lantern-net`), a Tauri
shell (`lantern-app`), and a headless CLI (`lantern-cli`).

This document is deliberately forward-looking. The near-term slices already
tracked in `CHANGELOG.md` (CLI subcommand expansion, more locales, ARM64,
MSIX hardening) stay as-is. What follows are **a few very ambitious
directions** — each one is a multi-milestone bet, written so we can argue
about it later, not a committed plan. Every proposal is measured against
Lantern's non-negotiable: **local-only by default, no telemetry, the network
is opt-in and quarantined in `lantern-net`.**

---

## Guiding principles (the tests every idea must pass)

1. **Local-only stays sacred.** Any new capability must work fully offline.
   Network features are opt-in, per-session, and compiled out of the offline
   flavour — enforced in CI, exactly as the dead-link checker is today.
2. **The core stays pure.** `lantern-core` remains I/O-free. New surface area
   pushes I/O into `lantern-io` and network into `lantern-net`.
3. **Everything the GUI does, the CLI can do.** Shared core APIs mean each
   feature lands as a library capability first, then a Tauri command and a CLI
   subcommand — never GUI-only.
4. **Reviewability is the product.** Destructive-by-default is banned; every
   change is a proposed, inspectable diff the user approves.

---

## 1. Lantern everywhere: one local-only core, five platforms

**Vision.** The same bookmark-hygiene experience on Windows, macOS, Linux, and
— the ambitious part — **iOS and Android**, where bookmark exports are hardest
to clean today.

**Why it fits.** `lantern-app` is already a `cdylib` and the repo carries
`icons/android` and `icons/ios`; Tauri 2 targets mobile. The pure core and the
strict I/O boundary mean the hard platform work is confined to `lantern-io`
(file access, settings paths) and the shell.

**Technical sketch.**
- Promote Linux/macOS from "compiles" to "shipped" (this repo now builds the
  full workspace on Linux; wire it into release CI alongside the Windows jobs).
- Abstract `lantern-io`'s settings/recent-files/recovery paths behind a
  platform trait; add mobile document-picker + share-sheet backends.
- Ship a mobile-first layout variant of the three-pane workspace (the panes
  already collapse cleanly; formalize a stacked navigation mode).

**Risks.** WebView2 is Windows-only (macOS uses WKWebView, Linux WebKitGTK,
mobile the system webview) — rendering parity and file-system sandboxing on
iOS are the real cost centers, not the core logic.

---

## 2. A sandboxed, community treatment ecosystem (WASM rule packs)

**Vision.** Anyone can write and share a sanitization treatment — "strip
$SHOP's session params", "de-mobilize $NEWS URLs" — **without recompiling
Lantern and without trusting arbitrary native code.**

**Why it fits.** Treatments are already a clean trait (`Treatment` in
`lantern-core/src/sanitize/treatments/`) with a built-in registry. The
contributor guide's worked example is literally "add a treatment." The missing
piece is a safe extension boundary.

**Technical sketch.**
- Define a stable treatment ABI and run third-party treatments as **WebAssembly
  components** in a capability-free sandbox (no I/O, no network — a perfect
  match for the "core is pure" rule). A treatment gets a node in, returns
  proposed `Change`s out.
- Ship a **local, signed rule-pack format** (`.lantern-rules` bundles of TOML +
  optional WASM). Distribution is file-based and offline-first: import a pack
  from disk, verify its signature, see exactly which fields it can touch before
  enabling it. No central server, no account.
- Extend the existing rule-set editor UI to browse, diff, and toggle packs.

**Risks.** ABI stability and a threat model for untrusted packs (even
sandboxed WASM can produce misleading diffs). Signing/trust UX must not become
a "click OK" ritual.

---

## 3. The "Pandoc for bookmarks": universal import/export

**Vision.** Lantern reads and writes *every* place bookmarks live, not just
Netscape HTML — and becomes the neutral, local converter between them.

**Why it fits.** The parser/model/emit split in `lantern-core` already
separates "understand a document" from "render a document." Adding formats is
additive, and it dramatically widens who Lantern helps.

**Technical sketch.**
- **Readers** (in `lantern-io`, read-only, never mutating the source): Chrome/
  Edge/Brave `Bookmarks` JSON, Firefox `places.sqlite`, Safari `plist`,
  Pocket/Raindrop/OneTab exports.
- **Writers**: Netscape HTML (today), JSON, Markdown link lists, CSV, and a
  round-trippable native format that preserves everything the model knows.
- A `lantern convert IN OUT` CLI verb makes this scriptable; the GUI gains an
  "Import from…" flow that still routes every change through the diff.

**Risks.** Reading live browser profile databases (locking, schema drift across
versions) is fiddly and must stay strictly read-only and clearly labeled to
preserve the privacy promise.

---

## 4. On-device intelligence: enrichment without the cloud

**Vision.** Smart bookmark curation — deduplication, near-duplicate detection,
auto-tagging, dead-link triage, stale-bookmark surfacing — **that never phones
home.**

**Why it fits.** This is the strongest expression of Lantern's thesis: the
features people assume require a cloud service, delivered entirely locally.

**Technical sketch.**
- **Offline, zero-network first:** exact + fuzzy duplicate detection (URL
  canonicalization already exists in the treatments), folder-structure health
  scores, and "you saved this 3 years ago and never revisited" heuristics from
  the timestamps in the model.
- **On-device auto-categorization:** a small embedded model (e.g. a quantized
  embedding model via a bundled runtime) suggests folders/tags from titles
  alone. Runs on the CPU, ships in the binary, produces *suggestions the user
  approves as diffs* — never silent moves.
- **Opt-in link intelligence** stays in `lantern-net` behind the `checker`
  feature: batch dead-link checking, redirect-chain flattening, and optional
  Tor/SOCKS routing for users who want checks without revealing their library.

**Risks.** Binary size and reproducible builds (the repo already runs a
reproducibility check) — an embedded model must not blow up the "small Windows
executable" identity. Keep it a separate build flavour if needed.

---

## 5. A versioned, mergeable local library (git for your bookmarks)

**Vision.** Your bookmark collection gains **history and a merge story** without
a server: browse how your library looked six months ago, undo across sessions,
and reconcile "laptop export vs. desktop export" as a first-class 3-way merge.

**Why it fits.** Cross-document merge already exists (ADR-0009,
`MergePickerModal`, `compare_tabs`), and undo restores state byte-for-byte. The
conceptual leap from "compare two open tabs" to "compare across time" is small
and squarely on-brand.

**Technical sketch.**
- A **content-addressed, append-only store** in `lantern-io` (local files only)
  snapshots each save; the model is small enough that full snapshots are cheap
  and diffs are trivial.
- Promote the existing diff engine to a **3-way merge** with the same
  reviewable-change UX, so merging two machines' exports is safe and auditable.
- **Optional, bring-your-own-storage, end-to-end-encrypted sync**: Lantern only
  ever reads/writes an encrypted blob to a folder *you* point at (Syncthing, a
  USB stick, your own cloud drive). No Lantern account, no Lantern server —
  consistent with "no call-home URLs exist anywhere in the codebase."

**Risks.** Encryption key management UX is the hard part; sync conflict
resolution must never resolve destructively without review.

---

## Sequencing (a suggestion, not a promise)

- **Foundation first:** #1 (ship Linux/macOS from the now-working build) and #3
  (import readers) widen reach and unlock the others.
- **Differentiators next:** #2 (treatment packs) and #4 (offline intelligence)
  are the features no cloud competitor can honestly match.
- **Long game:** #5 (versioned/mergeable library) reframes Lantern from a
  one-shot scrubber into a durable, private home for a bookmark collection.

Whatever we pick, the rule holds: it ships as a `lantern-core` capability with
a CLI verb and a reviewable diff, and it works with the network cable unplugged.
