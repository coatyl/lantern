# Security Policy

Lantern is a local-only bookmark hygiene tool. Reports that affect user data or the local-only guarantee get priority over feature work.

## Supported versions

Only the latest published release gets fixes. There are no backports before 1.0.

## Reporting a vulnerability

Email **security@lantern.dev** *(placeholder; TODO: replace with the real address once a project mailbox is registered)*.

Include, where you can:

- what the issue is and its impact;
- a minimal reproduction (a sample bookmark file is fine; redact anything you would not want shared);
- the Lantern version (**Settings → About**) and your Windows build;
- whether you would like credit in the release notes.

Do **not** open a public GitHub issue for security reports.

We aim to acknowledge a report within **5 business days** and to give an initial assessment within **10 business days**.

## Coordinated disclosure

We follow a **90-day** disclosure window from the date we acknowledge the report. Within that window we release a fix or publish an advisory with a mitigation. If we need longer, we agree a new timeline with you. Once a fix ships or the window closes, whichever comes first, you are free to publish.

## The local-only guarantee

Lantern has no telemetry, analytics, crash reporting or update checks. The only network code is the dead-link checker in the `lantern-net` crate, behind the `checker` Cargo feature. It refuses to run until the user enables it in **Settings → General**, and the offline build (`--no-default-features`) does not compile it at all. CI fails if `reqwest` or `hyper` appear in the offline build's dependency graph.

A vulnerability that breaks this guarantee (leaking user data over the network, writing files the user did not ask for, loading remote code) is the highest-severity class. Please say so in your report.

## Verifying a download

Every release carries `SHA256SUMS.txt`. Compare it with:

```powershell
(Get-FileHash -Algorithm SHA256 .\lantern-v<ver>-portable-x64.exe).Hash
```

Releases are **not Authenticode-signed yet**: the signing pipeline exists in CI but has no certificate. **Settings → About** reports `unsigned` for these builds and `✓ signed` for signed ones. Once releases are signed, `Get-AuthenticodeSignature` on a downloaded binary should report `Status: Valid`. If a release is labelled signed and the binary reports `NotSigned`, treat it as tampered with and report it before running it.

## Out of scope

- Vulnerabilities in third-party dependencies that are already disclosed upstream. Report those to the upstream project.
- Attacks that need physical access to an unlocked machine.
- Social engineering of project maintainers.
