# Lantern — philosophical bets

This is not a backlog. It is a set of **major changes to what Lantern *is***.
Each bet below is a fork in the road: if we take it, the product, the UI, and
the architecture all move. If we do not, we stay a very good one-shot HTML
scrubber — which is a finished thought, not a project.

The non-negotiables do not move:

1. Local-only by default. No telemetry. No accounts. No call-home.
2. The network, if it exists, is opt-in, per-session, and quarantined in
   `lantern-net` (compiled out of the offline flavour).
3. Every mutation is a proposed, reviewable diff. Nothing silent.
4. `lantern-core` stays I/O-free. The GUI and the CLI stay peers.

Everything else is on the table.

---

## Bet 1 — Stop being a file editor. Become a private archive.

**Today.** Lantern opens a `bookmarks.html`, lets you scrub it, writes a
clean copy, and forgets you. The file is the product. The app is a lens.

**The change.** The HTML file becomes an *import/export format*. The product
is a **local, versioned library** of everything you have ever saved: history,
merges across machines, search that remembers last month. Opening a file is
how a collection *enters* Lantern, not how you *use* Lantern.

This is the largest identity shift. It turns a utility into a place.

**What has to be true.** A content-addressed local store in `lantern-io`.
The existing cross-document merge and undo become history, not one-session
tricks. The library home that replaced the welcome screen is the first step.

**What we refuse.** A Lantern account. A Lantern server. Sync that is not
"a folder you pointed at, encrypted, yours."

---

## Bet 2 — Hygiene is a verb, not the product.

**Today.** The centre of gravity is sanitization treatments (UTM stripping,
affiliate params, email-in-title). That is a *feature*. We have been
treating it as the *thesis*.

**The change.** Treatments stay, and they stay reviewable — but they become
one instrument in a kit whose job is **curation**: duplicates, near-duplicates,
stale saves, dead links (opt-in), auto-suggested folders the user approves.
The question stops being "did we strip the tracker?" and becomes "do you still
want this, and where does it belong?"

**What has to be true.** A second class of *passes* next to treatments:
analysis passes that propose *structure* (merge these two, move this, drop
this) with the same diff UX. *Find duplicates* (exact URLs) is the first.
On-device only. No ranking model that phones home.

---

## Bet 3 — One core, many bodies. The desktop app is a client.

**Today.** Windows-first Tauri desktop is "the app." Linux/macOS compile.
The CLI is a companion. Mobile icons exist and go unused.

**The change.** `lantern-core` + `lantern-io` become a **local-first
protocol**. The Tauri window is client #1. The CLI is client #2. A local
loopback API (off by default, bind `127.0.0.1` only, token on disk) lets a
browser extension, a share sheet, or a future iOS/Android shell talk to the
same library without ever leaving the machine.

**What we refuse.** Any listener on a public interface. Any cloud relay.
"Lantern sync" that is not BYO-folder.

This is how we get to five platforms without five products.

---

## Bet 4 — The brand is a lantern, not an IDE skin.

**Today.** A first cut is in: warm ink and paper themes, a library home
instead of a logo-and-button welcome screen, empty states that say what to
do. Underneath it is still a three-pane developer tool, a cousin of a code
editor that happens to edit bookmarks.

**The change.** Lean into the name. Light in a dark room. Paper, warmth,
quiet craft. The archive should *feel* like an archive — not like VS Code
with a different file type. Typography, empty states, the library home, and
the diff review should all say "this is a private room for things you
saved," not "this is a pipeline."

**What has to be true.** A real design language (not just token tweaks):
type hierarchy, a light theme that is first-class (not an inversion),
motion that is almost none, empty states that teach. Accessibility stays
non-negotiable (the keyboard map, WCAG AA). Beauty is not a tax on
operability.

---

## Bet 5 — Extensions without trust. A capability-free economy.

**Today.** New sanitization rules mean a PR against `lantern-core`. That
does not scale, and it should not — we should not be the bottleneck for
"strip $SHOP's session params."

**The change.** Third-party treatments ship as **WebAssembly components**
with no I/O, no network, no clock. A node in, a list of proposed `Change`s
out. Distributed as signed, file-based packs you import from disk. No
marketplace server. You can read every byte.

**What we refuse.** Native plugins. Unsigned auto-update of packs. A
central registry that we operate.

---

## Bet 6 — Universal memory, not Netscape HTML.

**Today.** Netscape HTML and Chrome JSON in, Netscape HTML out. The rest of
the world's bookmarks (Firefox `places.sqlite`, Safari plist, Pocket,
Raindrop, OneTab, Markdown link dumps) are someone else's problem.

**The change.** Lantern is the **Pandoc of saved links**. Read anything,
write anything, the model in the middle is ours. `lantern-cli convert` is a
first-class verb. Live browser-profile reads are strictly read-only and
loudly labeled.

This is how we stop being a tool for people who already know how to export
`bookmarks.html`.

---

## How to argue about these

Take them in this order if we take them at all:

1. **Bet 4 (design)** and **Bet 6 (formats)** are the cheapest identity
   shifts — they do not require a store.
2. **Bet 2 (curation)** rides on the existing pass/diff machinery.
3. **Bet 1 (archive)** is the point of no return; do not start it as a
   skin. Start it as a store.
4. **Bet 3 (protocol)** and **Bet 5 (WASM)** are how the project outlives
   a single binary.

A slice that cannot be expressed as a `lantern-core` capability, a CLI
verb, and a reviewable diff is not a Lantern slice.

Agents working these bets should ship a **thin, real, reviewable first
cut** — not a manifesto. This file is the manifesto. The PRs are the cuts.
