//! `UiError`: the single error type that crosses the Tauri IPC boundary.
//!
//! Every command returns `Result<T, UiError>`.  Tauri serialises the error to
//! JSON and the UI pattern-matches on the `kind` discriminant to render the
//! right message.
//!
//! `UiError` intentionally does **not** expose raw filesystem paths or internal
//! Rust error chains to the UI; those go to the log file instead.

use serde::Serialize;

pub type CommandResult<T> = Result<T, UiError>;

/// All error conditions that can surface to the UI layer.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum UiError {
    /// A filesystem read or write failed.
    Io(String),
    /// The bookmark file could not be parsed (not Netscape HTML or Chrome JSON).
    Parse {
        line: u32,
        column: u32,
        reason: String,
    },
    /// The requested tab ID does not exist in the registry.
    TabNotFound(u64),
    /// A pending change set ID was not found (expired or already applied).
    ChangeSetNotFound(u64),
    /// An operation was requested that is not valid in the current state.
    InvalidOperation(String),
    /// An unexpected internal error (should be accompanied by a bug report).
    Internal(String),
}

impl std::fmt::Display for UiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
            Self::Parse {
                line,
                column,
                reason,
            } => {
                write!(f, "parse error at {line}:{column}: {reason}")
            }
            Self::TabNotFound(id) => write!(f, "tab {id} not found"),
            Self::ChangeSetNotFound(id) => {
                write!(f, "change set {id} not found or already applied")
            }
            Self::InvalidOperation(msg) => write!(f, "invalid operation: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

// Conversion helpers from crate-specific errors.

impl From<lantern_io::IoError> for UiError {
    fn from(e: lantern_io::IoError) -> Self {
        // Log the full error; only the category reaches the UI.
        Self::Io(e.to_string())
    }
}

impl From<lantern_core::CoreError> for UiError {
    fn from(e: lantern_core::CoreError) -> Self {
        match e {
            lantern_core::CoreError::ParseFailed {
                line,
                column,
                reason,
            } => Self::Parse {
                line,
                column,
                reason,
            },
            lantern_core::CoreError::InvalidOperation(msg) => Self::InvalidOperation(msg),
        }
    }
}
