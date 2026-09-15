//! Sanitization engine.
//!
//! Public entry points:
//! - [`pass::run_pass`]: execute a rule set over a document (read-only).
//! - [`Document::apply`]: commit approved changes and push an undo entry.
//! - [`Document::undo`] / [`Document::redo`]: navigate undo history.
//!
//! # Module layout
//!
//! ```text
//! sanitize/
//!   treatment.rs:     Treatment trait, Change, ChangeSet, PassContext
//!   pass.rs:          RuleSet, PassTarget, run_pass
//!   apply.rs:         Document::apply, Document::undo, Document::redo
//!   treatments/
//!     url_qp.rs:      url.qp.utm, url.qp.click_ids, url.qp.session
//!     title.rs:       title.whitespace
//! ```

pub mod apply;
pub mod diff;
pub mod pass;
pub mod treatment;
pub mod treatments;

pub use apply::ApplyError;
pub use diff::{char_diff, DiffSpan, DiffTag};
pub use pass::{run_pass, PassTarget, RuleSet};
pub use treatment::{
    Change, ChangeSet, PassContext, Treatment, TreatmentCategory, TreatmentConfig,
};
