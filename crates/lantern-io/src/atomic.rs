//! Atomic write helper.
//!
//! All writes in `lantern-io` go through [`write_atomic`], which writes to a
//! `NamedTempFile` in the target directory and then calls `persist` (which
//! is an atomic rename on all platforms we care about).  A crash mid-write
//! therefore never leaves a half-written file at the destination path.

use std::io::Write as _;
use std::path::Path;

use crate::error::{IoError, Result};

/// Write `bytes` to `path` atomically (temp file + rename).
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    // Create the temp file in the same directory as the destination so that
    // the rename stays on one filesystem (cross-device rename would fail).
    let parent = path.parent().unwrap_or(Path::new("."));

    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|e| IoError::Write {
        path: path.to_owned(),
        source: e,
    })?;

    tmp.write_all(bytes).map_err(|e| IoError::Write {
        path: path.to_owned(),
        source: e,
    })?;

    tmp.persist(path).map_err(|e| IoError::Write {
        path: path.to_owned(),
        source: e.error,
    })?;

    Ok(())
}
