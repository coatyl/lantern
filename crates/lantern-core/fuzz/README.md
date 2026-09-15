# Fuzz targets

`cargo-fuzz` harnesses for `lantern-core`.  Run manually on a nightly
toolchain; CI does not exercise these.

## Setup (one time)

```bash
rustup toolchain install nightly
cargo install cargo-fuzz
```

## Run a target

```bash
cd crates/lantern-core/fuzz

# Parser: feeds arbitrary bytes at the Netscape HTML parser.
cargo +nightly fuzz run fuzz_parser

# URL treatments: pairs arbitrary URLs with every URL-mutating treatment.
cargo +nightly fuzz run fuzz_url_treatments
```

Stop with `Ctrl+C` once you've run enough iterations (typically a few minutes
for smoke testing, hours for a proper campaign).

## Adding a new target

1. Drop a `fuzz_targets/my_target.rs` file that pulls in `libfuzzer-sys` and
   wraps the code under test in `fuzz_target!`.
2. Add a `[[bin]]` entry in `Cargo.toml` pointing at the new file.

## Why this is a separate crate

`cargo-fuzz` takes over the crate's build profile (nightly-only sanitizers,
custom linker flags) and expects to own the workspace.  Keeping the harnesses
out of the main workspace avoids polluting the normal `cargo build` /
`cargo test` flow with nightly requirements.
