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

    /// `write_bookmark_file` was pointed at the file the document was read
    /// from; Lantern never modifies the original.
    #[error("refusing to overwrite the source file {0}; export to a different path")]
    SourceFileOverwrite(PathBuf),

    /// A rule-set file names a treatment ID the registry does not know.
    #[error("unknown treatment id \"{0}\"")]
    UnknownTreatment(String),

    /// A treatment's `configure()` rejected its config block.
    #[error("invalid config for treatment \"{id}\": {reason}")]
    TreatmentConfig { id: String, reason: String },

    /// Shipped rule sets cannot be deleted (the next seed would recreate them).
    #[error("refusing to delete built-in rule set \"{0}\"")]
    BuiltinRuleSet(String),

    /// A rule set with this name already exists in the store.
    #[error("rule set \"{0}\" already exists")]
    RuleSetExists(String),
}

pub type Result<T> = std::result::Result<T, IoError>;
