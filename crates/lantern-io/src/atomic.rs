//! Atomic write helper shared by every writer in this crate.

use std::io::Write as _;
use std::path::Path;

use crate::error::{IoError, Result};

/// Write `bytes` to `path` via a temp file in the same directory followed by
/// a rename, so a crash mid-write never leaves a truncated file at `path`.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let write_err = |source| IoError::Write {
        path: path.to_owned(),
        source,
    };

    // Same directory as the destination: a cross-device rename would fail.
    let parent = path.parent().unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(write_err)?;
    tmp.write_all(bytes).map_err(write_err)?;
    tmp.persist(path).map_err(|e| write_err(e.error))?;
    Ok(())
}
