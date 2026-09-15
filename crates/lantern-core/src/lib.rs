pub mod diff;
pub mod emit;
pub mod error;
pub mod model;
pub mod parser;
pub mod sanitize;
pub mod search;

pub use error::{CoreError, Result};
pub use search::{IndexedDoc, SearchIndex};
