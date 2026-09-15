use thiserror::Error;

/// The error type for all `lantern-core` operations.
///
/// Individual sub-crates wrap these into their own error types; the app layer
/// collects everything into a `UiError` for the frontend.
#[derive(Debug, Error)]
pub enum CoreError {
    /// The HTML parser could not make sense of the input even with tolerant
    /// handling. This is the path for files that are not bookmark HTML at all.
    #[error("parse failed at line {line}, column {column}: {reason}")]
    ParseFailed {
        line: u32,
        column: u32,
        reason: String,
    },

    /// A caller passed arguments that violate an invariant (e.g. an unknown NodeId).
    #[error("invalid operation: {0}")]
    InvalidOperation(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
