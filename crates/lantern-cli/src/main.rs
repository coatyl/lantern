//! `lantern-cli`: command-line companion to the Lantern desktop app.
//!
//! Thin entry point that delegates to [`cli::run`].  All command-tree
//! definitions and execution logic live in `cli.rs` so they are testable
//! without spawning a subprocess.

mod cli;

fn main() -> anyhow::Result<()> {
    cli::run(std::env::args_os())
}
