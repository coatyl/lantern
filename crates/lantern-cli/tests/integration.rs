//! End-to-end coverage of the `lantern` binary via `assert_cmd`.
//!
//! These tests spawn the real binary so they exercise clap parsing,
//! the full lantern-core / lantern-io integration, and stdout output
//! exactly as a user would experience them.

use std::path::PathBuf;
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("small.html")
}

fn chrome_json_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("chrome-bookmarks.json")
}

fn duplicates_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("duplicates.html")
}

fn lantern() -> Command {
    Command::cargo_bin("lantern").expect("lantern binary builds in this crate")
}

#[test]
fn version_reports_workspace_version() {
    lantern()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn info_reports_nonzero_counts_for_fixture() {
    let fixture = fixture_path();
    let assert = lantern().arg("info").arg(&fixture).assert().success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(output.contains("bookmarks:"), "stdout was: {output}");
    assert!(output.contains("folders:"), "stdout was: {output}");
    assert!(output.contains("max depth:"), "stdout was: {output}");

    // Verify the bookmark + folder counts are non-zero.  We parse the
    // numeric column from each line rather than hard-coding fixture
    // counts so the fixture can grow without bricking the test.
    let bookmarks = parse_count(&output, "bookmarks:");
    let folders = parse_count(&output, "folders:");
    assert!(
        bookmarks > 0,
        "expected non-zero bookmarks, got {bookmarks}"
    );
    assert!(folders > 0, "expected non-zero folders, got {folders}");
}

#[test]
fn sanitize_dry_run_prints_summary_without_writing_output() {
    let fixture = fixture_path();
    let tmp = tempfile::tempdir().unwrap();
    let phantom_output = tmp.path().join("must-not-exist.html");

    lantern()
        .arg("sanitize")
        .arg(&fixture)
        .arg("-o")
        .arg(&phantom_output)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("dry run"))
        .stdout(predicate::str::contains("proposed changes:"))
        .stdout(predicate::str::contains("no output file written"));

    assert!(
        !phantom_output.exists(),
        "dry run unexpectedly wrote {}",
        phantom_output.display()
    );
}

#[test]
fn sanitize_writes_valid_bookmark_file() {
    let fixture = fixture_path();
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("clean.html");

    lantern()
        .arg("sanitize")
        .arg(&fixture)
        .arg("-o")
        .arg(&output)
        .assert()
        .success()
        .stdout(predicate::str::contains("wrote"));

    assert!(output.exists(), "output file should be written");

    // Round-trip: parse the emitted file and confirm we still get a
    // structurally valid document with at least one bookmark.
    let bytes = std::fs::read(&output).unwrap();
    let doc = lantern_core::parser::parse(&bytes).expect("emitted file must re-parse");
    assert!(
        doc.stats.bookmark_count > 0,
        "round-tripped file lost all bookmarks"
    );
    assert!(
        doc.stats.folder_count > 0,
        "round-tripped file lost all folders"
    );
}

#[test]
fn info_accepts_chrome_bookmarks_json() {
    let fixture = chrome_json_fixture();
    let assert = lantern().arg("info").arg(&fixture).assert().success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(parse_count(&output, "bookmarks:"), 3);
    assert_eq!(parse_count(&output, "folders:"), 3);
    assert_eq!(parse_count(&output, "max depth:"), 2);
}

#[test]
fn convert_chrome_json_emits_netscape_html() {
    let fixture = chrome_json_fixture();
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("bookmarks.html");

    lantern()
        .arg("convert")
        .arg(&fixture)
        .arg("-o")
        .arg(&output)
        .assert()
        .success()
        .stdout(predicate::str::contains("wrote"));

    let emitted = std::fs::read_to_string(&output).unwrap();
    assert!(
        emitted.contains("<!DOCTYPE NETSCAPE-Bookmark-file-1>"),
        "convert must emit Netscape HTML, got:\n{emitted}"
    );
    assert!(
        emitted.contains("utm_source=newsletter"),
        "UTM params must survive convert so sanitize can still strip them"
    );

    let doc = lantern_core::parser::parse(emitted.as_bytes()).expect("emitted HTML must re-parse");
    assert_eq!(doc.stats.bookmark_count, 3);
    assert_eq!(doc.stats.folder_count, 3);
}

#[test]
fn convert_then_sanitize_strips_utm() {
    let fixture = chrome_json_fixture();
    let tmp = tempfile::tempdir().unwrap();
    let converted = tmp.path().join("converted.html");
    let cleaned = tmp.path().join("cleaned.html");

    lantern()
        .arg("convert")
        .arg(&fixture)
        .arg("-o")
        .arg(&converted)
        .assert()
        .success();

    lantern()
        .arg("sanitize")
        .arg(&converted)
        .arg("-o")
        .arg(&cleaned)
        .arg("--rule-set")
        .arg("minimal-clean")
        .assert()
        .success();

    let cleaned_html = std::fs::read_to_string(&cleaned).unwrap();
    assert!(
        !cleaned_html.contains("utm_source"),
        "minimal-clean should strip UTM params after convert:\n{cleaned_html}"
    );
    assert!(
        cleaned_html.contains("https://example.com/"),
        "the bookmark URL itself must remain"
    );
}

#[test]
fn convert_html_round_trips() {
    let fixture = fixture_path();
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("copy.html");

    lantern()
        .arg("convert")
        .arg(&fixture)
        .arg("-o")
        .arg(&output)
        .assert()
        .success();

    let src = lantern_core::parser::parse(&std::fs::read(&fixture).unwrap()).unwrap();
    let dst = lantern_core::parser::parse(&std::fs::read(&output).unwrap()).unwrap();
    assert_eq!(src.stats.bookmark_count, dst.stats.bookmark_count);
    assert_eq!(src.stats.folder_count, dst.stats.folder_count);
}

#[test]
fn rule_sets_lists_shipped_builtins() {
    lantern()
        .arg("rule-sets")
        .assert()
        .success()
        .stdout(predicate::str::contains("Minimal clean"))
        .stdout(predicate::str::contains("Aggressive scrub"))
        .stdout(predicate::str::contains("Full scrub"))
        .stdout(predicate::str::contains("Find duplicates"));
}

#[test]
fn sanitize_find_duplicates_dry_run_lists_proposed_deletes() {
    let fixture = duplicates_fixture_path();
    let tmp = tempfile::tempdir().unwrap();
    let phantom_output = tmp.path().join("must-not-exist.html");

    let assert = lantern()
        .arg("sanitize")
        .arg(&fixture)
        .arg("-o")
        .arg(&phantom_output)
        .arg("--rule-set")
        .arg("find-duplicates")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "dry run: rule set \"Find duplicates\"",
        ))
        .stdout(predicate::str::contains("deletions: 1"))
        .stdout(predicate::str::contains("proposed deletions"))
        .stdout(predicate::str::contains("Newer copy"))
        .stdout(predicate::str::contains("https://example.com/same"))
        .stdout(predicate::str::contains("auto-approves"))
        .stdout(predicate::str::contains("no output file written"));

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        !output.contains("Query differs"),
        "query-param near-dupes must not be proposed; stdout was: {output}"
    );
    assert!(
        !output.contains("Unique bookmark"),
        "unique URLs must not be proposed; stdout was: {output}"
    );
    assert!(
        !phantom_output.exists(),
        "dry run unexpectedly wrote {}",
        phantom_output.display()
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Pull the integer that follows `prefix:` on the matching `info` line.
fn parse_count(stdout: &str, prefix: &str) -> u64 {
    for line in stdout.lines() {
        if let Some(rest) = line.trim_start().strip_prefix(prefix) {
            return rest
                .trim()
                .parse::<u64>()
                .unwrap_or_else(|_| panic!("could not parse count from line: {line:?}"));
        }
    }
    panic!("did not find line starting with {prefix:?} in stdout");
}
