//! Filesystem boundary for Lantern.
//!
//! This crate is the **only** place in the Lantern codebase that touches disk.
//! `lantern-core` is pure logic; `lantern-app` delegates all file I/O here.
//! Keeping the boundary narrow makes the I/O surface auditable and the core
//! logic trivially unit-testable without a filesystem.
//!
//! # Public API (TDD §6)
//!
//! ```text
//! read_bookmark_file(path)          → Document
//! write_bookmark_file(path, doc)    → ()       [atomic, source protected]
//! read_settings(path)               → Settings
//! write_settings(path, settings)    → ()       [atomic]
//! read_ruleset(path)                → RuleSet
//! write_ruleset(path, rs)           → ()       [atomic]
//! write_sidecar(path, sidecar)      → ()       [atomic, optional feature]
//! ```
//!
//! Every function is synchronous.  Threading is handled by the Tauri command
//! layer via `tokio::task::spawn_blocking`.

mod atomic;

pub mod bookmark;
pub mod error;
pub mod logstore;
pub mod ruleset;
pub mod rulestore;
pub mod settings;
pub mod sidecar;

// ---------------------------------------------------------------------------
// Flat re-exports for the most common entry points
// ---------------------------------------------------------------------------

pub use bookmark::{read_bookmark_file, write_bookmark_file};
pub use error::{IoError, Result};
pub use logstore::{log_path, read_recent, RawLogLine};
pub use ruleset::{build_ruleset, read_ruleset, write_ruleset, RuleSetSpec};
pub use settings::{read_settings, write_settings, ListDensity, Settings, Theme};
pub use sidecar::{write_sidecar, Sidecar};
