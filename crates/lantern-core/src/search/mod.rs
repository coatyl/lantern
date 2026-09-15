//! v0.0.8 background search indexing baseline.  See [`index`] for the
//! [`SearchIndex`] type, the tokenisation rules, and the perf budget.

pub mod index;

pub use index::{IndexedDoc, SearchIndex};
