//! Command-tree definitions and dispatch for the `lantern-cli` binary.
//!
//! The CLI is intentionally minimal for v0.1.0: four subcommands that
//! cover the headless path the GUI exercises (parse → convert / run
//! rule set → emit).  Custom rule-set discovery from disk and
//! structural commands such as `merge` / `dedupe` / `diff` are
//! post-v0.1.0.
//!
//! The public entry point is [`run`].  It accepts an `args` iterator so
//! integration tests and embedders can drive the CLI without going
//! through `std::env::args_os` directly.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::{Args, Parser, Subcommand};

use lantern_core::emit::EmitOptions;
use lantern_core::model::ids::NodeId;
use lantern_core::model::node::{Folder, Node};
use lantern_core::sanitize::pass::{run_pass, PassTarget, RuleSet};
use lantern_core::sanitize::treatment::{Change, ChangeKind};
use lantern_io::rulestore::{builtin_rule_sets, is_builtin};
use lantern_io::{build_ruleset, read_bookmark_file, read_ruleset, write_bookmark_file};

// ---------------------------------------------------------------------------
// Command tree
// ---------------------------------------------------------------------------

/// Top-level CLI for the `lantern-cli` binary.
#[derive(Debug, Parser)]
#[command(
    name = "lantern-cli",
    version,
    about = "Sanitise, inspect, and convert bookmark files from the shell.",
    long_about = "Lantern's headless companion to the Tauri GUI.  \
                  Reuses lantern-core and lantern-io so the rule set + \
                  sanitization logic gets a second consumer.  Reads \
                  Netscape / Firefox HTML and Chrome Bookmarks JSON; \
                  writes Netscape HTML only."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Apply a rule set to a bookmark file.
    ///
    /// Without `--dry-run` the CLI auto-approves every proposed change
    /// (including destructive deletions such as Find duplicates) and
    /// writes the result.  Inspect first with `--dry-run`.
    Sanitize(SanitizeArgs),
    /// Print structural information about a bookmark file.
    Info(InfoArgs),
    /// Convert a bookmark file to Netscape HTML.
    Convert(ConvertArgs),
    /// List the built-in rule sets.
    #[command(name = "rule-sets")]
    RuleSets(RuleSetsArgs),
}

#[derive(Debug, Args)]
struct SanitizeArgs {
    /// Path to a Netscape HTML or Chrome Bookmarks JSON file.
    input: PathBuf,

    /// Output file (defaults to `<input>.clean.html`).
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Built-in rule set to apply.  Default: `minimal-clean`.
    ///
    /// Accepts the slug (`minimal-clean`, `find-duplicates`) or the
    /// display name (`Minimal clean`, `Find duplicates`).  Mutually
    /// exclusive with `--rule-set-file`.
    #[arg(long, value_name = "NAME", conflicts_with = "rule_set_file")]
    rule_set: Option<String>,

    /// Path to a `.lantern-rules.toml` file.
    #[arg(long, value_name = "PATH")]
    rule_set_file: Option<PathBuf>,

    /// Print a summary of proposed changes without writing the output.
    ///
    /// A real (non-dry-run) sanitize auto-approves every proposed change,
    /// including destructive deletions.  Always inspect Find duplicates
    /// with `--dry-run` before applying.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct InfoArgs {
    /// Path to a Netscape HTML or Chrome Bookmarks JSON file.
    input: PathBuf,
}

#[derive(Debug, Args)]
struct ConvertArgs {
    /// Path to a Netscape HTML or Chrome Bookmarks JSON file.
    input: PathBuf,

    /// Destination Netscape HTML path. Required so the source is never
    /// overwritten (Chrome profile `Bookmarks` files stay read-only).
    #[arg(short, long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct RuleSetsArgs {
    /// List the built-in rule sets and exit (default behaviour for v0.1.0).
    #[arg(long)]
    list_builtin: bool,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse `args` and execute the requested subcommand.
///
/// Errors bubble up as `anyhow::Error` so the binary's `main` only has to
/// print them.  Tests use [`run`] directly to bypass clap's process-exit
/// behaviour for argument errors when convenient.
pub fn run<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    match cli.command {
        Command::Sanitize(a) => run_sanitize(a),
        Command::Info(a) => run_info(a),
        Command::Convert(a) => run_convert(a),
        Command::RuleSets(a) => run_rule_sets(a),
    }
}

// ---------------------------------------------------------------------------
// `sanitize`
// ---------------------------------------------------------------------------

fn run_sanitize(args: SanitizeArgs) -> Result<()> {
    let mut doc = read_bookmark_file(&args.input)
        .with_context(|| format!("reading {}", args.input.display()))?;

    let rule_set = resolve_rule_set(args.rule_set.as_deref(), args.rule_set_file.as_deref())?;

    // Run the pass and approve every proposed change.  In the GUI, the
    // user toggles `approved` per-row; the CLI's contract is "apply the
    // whole rule set as written", so we flip them all on.
    let mut cs = run_pass(&doc, &rule_set, PassTarget::AllNodes);
    let total = cs.changes.len();
    let (set_field, delete_node, set_flag) = count_kinds(&cs.changes);

    if args.dry_run {
        print_change_summary(
            &rule_set.name,
            total,
            set_field,
            delete_node,
            set_flag,
            &doc.root,
            &cs.changes,
        );
        return Ok(());
    }

    for c in &mut cs.changes {
        c.approved = true;
    }
    let applied = cs.changes.iter().filter(|c| c.approved).count();
    doc.apply(&cs)
        .with_context(|| format!("applying rule set \"{}\"", rule_set.name))?;

    let output = args
        .output
        .clone()
        .unwrap_or_else(|| default_output_path(&args.input));

    write_bookmark_file(&output, &doc, &EmitOptions::default())
        .with_context(|| format!("writing {}", output.display()))?;

    println!(
        "applied {applied} change{} (rule set \"{}\")",
        if applied == 1 { "" } else { "s" },
        rule_set.name,
    );
    println!("wrote {}", output.display());
    Ok(())
}

fn count_kinds(changes: &[lantern_core::sanitize::treatment::Change]) -> (usize, usize, usize) {
    let mut set_field = 0;
    let mut delete_node = 0;
    let mut set_flag = 0;
    for c in changes {
        match c.kind {
            ChangeKind::SetField { .. } => set_field += 1,
            ChangeKind::DeleteNode => delete_node += 1,
            ChangeKind::SetFlag { .. } => set_flag += 1,
        }
    }
    (set_field, delete_node, set_flag)
}

fn print_change_summary(
    rule_set_name: &str,
    total: usize,
    set_field: usize,
    delete_node: usize,
    set_flag: usize,
    root: &Folder,
    changes: &[Change],
) {
    println!("dry run: rule set \"{rule_set_name}\"");
    println!(
        "  proposed changes: {total} (field edits: {set_field}, \
         deletions: {delete_node}, flag updates: {set_flag})"
    );
    if delete_node > 0 {
        println!("  proposed deletions (destructive; not applied in dry-run):");
        for change in changes {
            if !matches!(change.kind, ChangeKind::DeleteNode) {
                continue;
            }
            match find_bookmark_preview(root, change.node_id) {
                Some((title, url)) => {
                    println!("    - [{title}] {url}");
                    println!("      {}", change.rationale);
                }
                None => {
                    println!("    - node {} ({})", change.node_id, change.rationale);
                }
            }
        }
    }
    println!(
        "  note: without --dry-run the CLI auto-approves every proposed \
         change, including deletions"
    );
    println!("  no output file written");
}

/// Title + URL for a dry-run deletion line.  Folders and missing IDs
/// return `None` so the caller can fall back to the node id.
fn find_bookmark_preview(folder: &Folder, node_id: NodeId) -> Option<(String, String)> {
    for child in &folder.children {
        match child {
            Node::Bookmark(b) if b.id == node_id => {
                return Some((b.title.clone(), b.url.as_str().to_owned()));
            }
            Node::Folder(f) => {
                if let Some(found) = find_bookmark_preview(f, node_id) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

fn default_output_path(input: &Path) -> PathBuf {
    // Append ".clean.html" to the *full* path so callers always end up
    // with a sibling file, not one with an exotic extension chain.
    let mut s: OsString = input.as_os_str().to_owned();
    s.push(".clean.html");
    PathBuf::from(s)
}

// ---------------------------------------------------------------------------
// `info`
// ---------------------------------------------------------------------------

fn run_info(args: InfoArgs) -> Result<()> {
    let doc = read_bookmark_file(&args.input)
        .with_context(|| format!("reading {}", args.input.display()))?;

    let depth = max_depth(&doc.root);
    let root_name = if doc.root.name.is_empty() {
        "(unnamed)"
    } else {
        doc.root.name.as_str()
    };

    println!("file:        {}", args.input.display());
    println!("root folder: {root_name}");
    println!("bookmarks:   {}", doc.stats.bookmark_count);
    println!("folders:     {}", doc.stats.folder_count);
    println!("separators:  {}", doc.stats.separator_count);
    println!("max depth:   {depth}");
    Ok(())
}

/// Tree depth in folder-edges from the root.
///
/// A document with bookmarks only at the top level has depth 0; one
/// nested folder containing bookmarks has depth 1; and so on.
fn max_depth(root: &Folder) -> u32 {
    fn walk(folder: &Folder, current: u32) -> u32 {
        let mut max = current;
        for child in &folder.children {
            if let Node::Folder(f) = child {
                let sub = walk(f, current + 1);
                if sub > max {
                    max = sub;
                }
            }
        }
        max
    }
    walk(root, 0)
}

// ---------------------------------------------------------------------------
// `convert`
// ---------------------------------------------------------------------------

fn run_convert(args: ConvertArgs) -> Result<()> {
    let doc = read_bookmark_file(&args.input)
        .with_context(|| format!("reading {}", args.input.display()))?;

    write_bookmark_file(&args.output, &doc, &EmitOptions::default())
        .with_context(|| format!("writing {}", args.output.display()))?;

    println!("wrote {}", args.output.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// `rule-sets`
// ---------------------------------------------------------------------------

fn run_rule_sets(_args: RuleSetsArgs) -> Result<()> {
    // `--list-builtin` is the only behaviour today; the flag is reserved
    // so future versions can add `--list-user <DIR>` without breaking the
    // command surface.
    println!("Built-in rule sets:");
    for (name, treatment_ids) in builtin_rule_sets() {
        let description = describe_builtin(name);
        println!(
            "  {:<18} {} ({} treatment{})",
            name,
            description,
            treatment_ids.len(),
            if treatment_ids.len() == 1 { "" } else { "s" },
        );
    }
    Ok(())
}

/// One-line description for each shipped built-in.
fn describe_builtin(name: &str) -> &'static str {
    match name {
        "Minimal clean" => "strip common UTM / click-id / tracking-fragment params, normalise whitespace",
        "Aggressive scrub" => "Minimal clean + session/affiliate/search params, fragment removal, email/handle scrubbing",
        "Full scrub" => "Aggressive scrub + path user segments, host demobilisation, shortener detection, author suffix",
        "Find duplicates" => {
            "propose deleting exact-URL duplicates, keeping the oldest (or first-seen); \
             dry-run to review — a real run auto-approves deletions"
        }
        _ => "(custom)",
    }
}

// ---------------------------------------------------------------------------
// Rule-set resolution
// ---------------------------------------------------------------------------

/// Resolve the rule set selected by the user.
///
/// Precedence (mirrors clap's `conflicts_with`):
/// 1. `--rule-set-file PATH` → load the TOML directly.
/// 2. `--rule-set NAME` → look up by display name or slug among built-ins.
/// 3. Neither → default to the canonical "Minimal clean".
fn resolve_rule_set(name: Option<&str>, file: Option<&Path>) -> Result<RuleSet> {
    if let Some(path) = file {
        return read_ruleset(path)
            .with_context(|| format!("reading rule set from {}", path.display()));
    }

    let requested = name.unwrap_or("Minimal clean");
    let canonical = canonicalise_builtin_name(requested).ok_or_else(|| {
        anyhow!(
            "unknown rule set \"{requested}\"; pass --rule-set-file <PATH> for custom sets, \
             or use one of: {}",
            builtin_rule_sets()
                .iter()
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
                .join(", "),
        )
    })?;

    // Build directly from the in-memory catalogue rather than touching disk.
    let (_, ids) = builtin_rule_sets()
        .into_iter()
        .find(|(n, _)| *n == canonical)
        .expect("canonicalise_builtin_name only returns names from builtin_rule_sets()");

    build_ruleset(canonical.to_string(), ids)
        .with_context(|| format!("building built-in rule set \"{canonical}\""))
}

/// Match a user-supplied rule-set name against the built-in catalogue.
///
/// Accepts both the display name (`"Minimal clean"`) and the slug
/// (`"minimal-clean"`), case-insensitively.  Returns the canonical
/// display name when matched, or `None` if no built-in matches.
fn canonicalise_builtin_name(input: &str) -> Option<&'static str> {
    let trimmed = input.trim();
    if is_builtin(trimmed) {
        for (name, _) in builtin_rule_sets() {
            if *name == *trimmed {
                return Some(name);
            }
        }
    }
    let normalised = trimmed.to_ascii_lowercase();
    for (name, _) in builtin_rule_sets() {
        if name.eq_ignore_ascii_case(trimmed) {
            return Some(name);
        }
        if lantern_io::rulestore::slugify(name) == normalised {
            return Some(name);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_output_path_appends_suffix() {
        let p = default_output_path(Path::new("foo.html"));
        assert_eq!(p, PathBuf::from("foo.html.clean.html"));
    }

    #[test]
    fn canonicalise_accepts_display_name_and_slug() {
        assert_eq!(
            canonicalise_builtin_name("Minimal clean"),
            Some("Minimal clean")
        );
        assert_eq!(
            canonicalise_builtin_name("minimal-clean"),
            Some("Minimal clean")
        );
        assert_eq!(
            canonicalise_builtin_name("MINIMAL CLEAN"),
            Some("Minimal clean")
        );
        assert_eq!(
            canonicalise_builtin_name("Aggressive scrub"),
            Some("Aggressive scrub")
        );
        assert_eq!(canonicalise_builtin_name("full-scrub"), Some("Full scrub"));
        assert_eq!(
            canonicalise_builtin_name("find-duplicates"),
            Some("Find duplicates")
        );
        assert_eq!(
            canonicalise_builtin_name("Find duplicates"),
            Some("Find duplicates")
        );
        assert_eq!(canonicalise_builtin_name("nope"), None);
    }
}
