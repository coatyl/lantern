//! Fuzz target for the Netscape bookmark HTML parser.
//!
//! Feeds arbitrary bytes to `lantern_core::parser::parse` and asserts that
//! the parser either returns `Ok` or `Err`: it must never panic, overflow,
//! or abort.  Run with:
//!
//! ```bash
//! cd crates/lantern-core/fuzz
//! cargo +nightly fuzz run fuzz_parser
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Discard the Result; we only care that the call returns without
    // panicking.  Out-of-memory is ignored (libfuzzer treats it as a
    // non-crash failure mode).
    let _ = lantern_core::parser::parse(data);
});
