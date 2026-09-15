//! `IoError`: all errors that can originate from disk operations.

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum IoError {
    #[error("I/O error reading {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("I/O error writing {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("bookmark parse error in {path}: {source}")]
    BookmarkParse {
        path: PathBuf,
        source: lantern_core::CoreError,
    },

    #[error("TOML deserialisation error in {path}: {reason}")]
    TomlDe { path: PathBuf, reason: String },

    #[error("TOML serialisation error: {0}")]
    TomlSer(String),

    #[error("JSON serialisation error: {0}")]
    JsonSer(String),

    /// Raised by `write_bookmark_file` when the destination path is the same
    /// file the document was read from (PRD F-EXP-7).
    #[error("refusing to overwrite the source file {0}; export to a different path")]
    SourceFileOverwrite(PathBuf),

    /// Raised by `read_ruleset` when a treatment ID in the TOML file is not
    /// recognised.
    #[error("unknown treatment id \"{0}\"")]
    UnknownTreatment(String),

    /// Raised when a treatment's `configure()` call rejects the provided
    /// config block.
    #[error("invalid config for treatment \"{id}\": {reason}")]
    TreatmentConfig { id: String, reason: String },
}

pub type Result<T> = std::result::Result<T, IoError>;
