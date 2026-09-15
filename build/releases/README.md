# target/ and build/releases

What a build writes, what this folder is for, and the v1.0.0 artefacts that existed on disk before the working copy left `Build\Lantern`.

## Contents

- [target/](#target)
- [This folder](#this-folder)
- [v1.0.0 artefacts (2026-05-06)](#v100-artefacts-2026-05-06)
- [How to rebuild](#how-to-rebuild)

## target/

`target/` sits at the repo root. Cargo owns it. Daily work fills `target/debug/`. `cargo build --release` and `npm run build` fill `target/release/`, including `lantern.exe`. When bundling is on, Tauri also writes installers under `target/release/bundle/`.

None of that is source. It regenerates from a cold checkout. The repo gitignores `/target/` so a clone does not carry it and a commit cannot pick it up by accident.

On the machine that built v1.0.0, four ship files were copied into `target/lantern-v1.0.0/` on 2026-05-06. That directory is still `target/`, so it was never going to be committed. The record of those four files is this page.

## This folder

`build/releases/` is documentation of what shipped. It does not hold the binaries. Rebuild from source; the hashes below are how you check a rebuild against the 2026-05-06 copies.

## v1.0.0 artefacts (2026-05-06)

Local path at the time: `target/lantern-v1.0.0/`.

| File | Bytes | SHA256 |
|---|---|---|
| `lantern-v1.0.0-installer-nsis-x64.exe` | 4410994 | `2ECA73F358086458F7E59D4705960C5B32562A3FF28C329368966225C4C9E05A` |
| `lantern-v1.0.0-installer-x64.msi` | 6180864 | `258E9796CC678578C0E3C6E5D43051125A797A25FA61D3EDF72C26BDE25FFD8E` |
| `lantern-v1.0.0-offline-x64.exe` | 13172736 | `E6627D6E776A6C07364799F7325737E4154C7DA624018DF0AA8492CCE8714572` |
| `lantern-v1.0.0-portable-x64.exe` | 16204288 | `4BA1F0EADF85E2E42FF5E4CAF5D8C90CE53B7129D4EE44EA3EDFD7F747A8835D` |

These were unsigned local builds. `Get-AuthenticodeSignature` on them reports unsigned. A later signed release replaces them; it does not rewrite this table.

## How to rebuild

From a clone of this repo, with the toolchain in `rust-toolchain.toml` and Node 20+:

```powershell
npm install
npm run build
```

That yields `target/release/lantern.exe`. The portable flavour is that executable. The offline flavour is:

```powershell
cargo build --release -p lantern-app --no-default-features --locked
```

NSIS and MSI come from Tauri bundle configs in `crates/lantern-app/` when `bundle.active` is true. Hash the results against the table above only if you are reproducing the 2026-05-06 build; a newer toolchain will not match.
