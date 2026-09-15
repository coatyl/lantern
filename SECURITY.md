# Security Policy

Lantern is a local-only bookmark hygiene tool.  We take the
"local-only" promise seriously and treat security reports as a high
priority.

## Supported versions

| Version | Supported          |
|---------|--------------------|
| 0.1.x   | Yes                |
| < 0.1   | No (please upgrade)|

Older releases will not receive backports.  The supported window will
expand once Lantern reaches 1.0.

## Reporting a vulnerability

Please report suspected vulnerabilities by email to:

**security@lantern.dev**  *(placeholder; TODO: replace with the real
address once a project mailbox is registered)*

Include, where possible:

- A description of the issue and its impact.
- A minimal reproduction (a sample bookmark file is fine; please
  redact anything you would not want shared).
- The Lantern version (`Settings → About`) and your Windows build.
- Whether you would like credit in the release notes.

Do **not** open a public GitHub issue for security reports.

## Acknowledgement and triage

- We aim to acknowledge new reports within **5 business days**.
- We aim to provide an initial triage assessment within **10 business
  days**.
- Critical issues affecting user data or the local-only guarantee are
  prioritised over feature work.

## Coordinated disclosure

We follow a **90-day coordinated-disclosure window**, measured from
the date we acknowledge the report:

- Within 90 days, we will release a fix or publish a public advisory
  with a documented mitigation.
- If we need additional time, we will request an extension and agree
  on a revised timeline with the reporter.
- After the window closes (or a fix ships, whichever is sooner), the
  reporter is free to publish independently.

## Privacy commitment

Lantern is **local-only by default** (PRD §9.2 / NFR-PR-1 through
NFR-PR-5).  No telemetry, no analytics, no crash-reporting service,
no update pings: none of these exist anywhere in the codebase.  The
optional dead-link checker is the only network-touching feature, is
opt-in per session, and is omitted entirely from the offline build
flavor at compile time (see `private/docs/03-technical-design.md`
§7, §12).

If a vulnerability would compromise this guarantee (for example by
causing the app to leak user data over the network, write outside its
sandbox, or load remote code), please flag that explicitly in your
report.  We treat those as the highest-severity class.

## Verifying release authenticity

From v0.0.10 onward, Windows release binaries (portable `lantern.exe`,
NSIS installer, MSIX bundle) are **Authenticode-signed** when a
code-signing certificate is available to the release pipeline.

To verify a downloaded binary:

```powershell
Get-AuthenticodeSignature .\lantern.exe
```

The output should report `Status: Valid` and a publisher name matching
the certificate's CN.  The About pane (`Settings → About → Signed`)
also reports `✓ signed` for signed builds; unsigned dev builds and
self-built binaries report `unsigned`.

If `Get-AuthenticodeSignature` reports `NotSigned` on a binary you
downloaded from a release labelled "signed," treat the binary as
suspect; it may have been tampered with in transit.  Report it via
the channel above before running it.

**Note (v0.0.10):** the signing pipeline is in place but ships
unsigned until the project's first Authenticode certificate is
acquired.  Track progress in
[`private/docs/STATUS.md`](private/docs/STATUS.md) "Next thing to do".

## Out of scope

- Issues in third-party dependencies that have already been disclosed
  upstream and have an open advisory; please report those to the
  upstream project.  We do track them via `cargo audit` /
  `cargo deny` in CI.
- Theoretical attacks that require physical access to an unlocked
  machine.
- Social-engineering attacks against project maintainers.

Thank you for helping keep Lantern users safe.
