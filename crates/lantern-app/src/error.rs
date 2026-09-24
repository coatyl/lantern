//! `UiError`: the error every command returns.
//!
//! It serialises as `{ "kind": ..., "detail": ... }` so the UI can pick a
//! message by `kind`.

use serde::Serialize;

pub type CommandResult<T> = Result<T, UiError>;

#[derive(Debug, Serialize, thiserror::Error)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum UiError {
    /// A filesystem read or write failed.
    #[error("I/O error: {0}")]
    Io(String),
    /// The bookmark file could not be parsed.
    #[error("parse error at {line}:{column}: {reason}")]
    Parse {
        line: u32,
        column: u32,
        reason: String,
    },
    #[error("tab {0} not found")]
    TabNotFound(u64),
    /// The change set was already applied, or its tab was closed.
    #[error("change set {0} not found or already applied")]
    ChangeSetNotFound(u64),
    #[error("invalid operation: {0}")]
    InvalidOperation(String),
    /// A bug: the operation should not have been able to fail.
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<lantern_io::IoError> for UiError {
    fn from(e: lantern_io::IoError) -> Self {
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
