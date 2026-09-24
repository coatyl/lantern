//! Filesystem boundary for Lantern.
//!
//! This is the only crate that touches disk: `lantern-core` stays pure logic,
//! and `lantern-app` / `lantern-cli` delegate all file I/O here, which keeps
//! the I/O surface small and auditable.
//!
//! Every function is synchronous; async callers run them on a blocking
//! thread.  Every write goes through a temp-file-plus-rename helper, so a
//! crash never leaves a half-written file at the destination.

mod atomic;

pub mod bookmark;
pub mod error;
pub mod logstore;
pub mod ruleset;
pub mod rulestore;
pub mod settings;

pub use bookmark::{read_bookmark_file, write_bookmark_file};
pub use error::{IoError, Result};
pub use logstore::{log_path, read_recent, RawLogLine};
pub use ruleset::{build_ruleset, read_ruleset, write_ruleset};
pub use settings::{read_settings, settings_dir, write_settings, ListDensity, Settings, Theme};
