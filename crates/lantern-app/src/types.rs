//! Serialisable view-model types that cross the Tauri IPC boundary.
//!
//! These are the shapes the UI receives from commands.  They are intentionally
//! flat and JSON-friendly; the richer domain types live in `lantern-core` and
//! are converted here.
//!
//! TypeScript counterparts live in `ui/src/ipc/types.ts`.  Run
//! `cargo test -p lantern-app export_bindings` to regenerate them from these
//! derives whenever types change.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ---------------------------------------------------------------------------
// Identifiers
// ---------------------------------------------------------------------------

pub type TabId = u64;
pub type NodeId = u64;
pub type ChangeSetId = u64;

// ---------------------------------------------------------------------------
// Tab / document
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct TabInfo {
    #[ts(type = "number")]
    pub id: TabId,
    /// Document title (from the bookmark file's `<TITLE>` element).
    pub title: String,
    /// Filesystem path the document was opened from, if any.
    pub path: Option<String>,
    pub dirty: bool,
    pub stats: DocStats,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct DocStats {
    #[ts(type = "number")]
    pub bookmark_count: u64,
    #[ts(type = "number")]
    pub folder_count: u64,
    #[ts(type = "number")]
    pub separator_count: u64,
}

// ---------------------------------------------------------------------------
// Tree pane
// ---------------------------------------------------------------------------

/// Recursive folder tree for the left pane.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct TreeView {
    pub root: TreeNode,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct TreeNode {
    #[ts(type = "number")]
    pub id: NodeId,
    pub name: String,
    pub children: Vec<TreeNode>,
}

/// Single tree row used by the v0.0.8 progressive tree expansion API.
///
/// Unlike [`TreeNode`], this struct does **not** carry its descendants;
/// the UI fetches them lazily through [`crate::commands::get_tree_children`]
/// when the user expands a folder.  The `has_children` flag tells the UI
/// whether to render an expand chevron for this row.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct TreeNodeLazy {
    #[ts(type = "number")]
    pub id: NodeId,
    pub name: String,
    /// `true` if this folder has any folder children that the UI should
    /// render an expand chevron for; `false` for leaves.  Children
    /// themselves are fetched lazily via `get_tree_children`.
    pub has_children: bool,
}

// ---------------------------------------------------------------------------
// List pane
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct ItemPage {
    pub items: Vec<FolderItem>,
    /// Total items in the folder before pagination.
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct FolderItem {
    #[ts(type = "number")]
    pub id: NodeId,
    pub kind: ItemKind,
    pub title: String,
    pub url: Option<String>,
    /// Registered domain (eTLD+1) extracted from the URL, if valid.
    pub domain: Option<String>,
    #[ts(type = "number | null")]
    pub add_date: Option<i64>,
    #[ts(type = "number | null")]
    pub last_modified: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Bookmark,
    Folder,
    Separator,
}

// ---------------------------------------------------------------------------
// Sorting (from UI → Rust)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SortSpec {
    pub column: SortColumn,
    #[serde(default)]
    pub descending: bool,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortColumn {
    Title,
    Url,
    Domain,
    AddDate,
    LastModified,
}

// ---------------------------------------------------------------------------
// Search (from UI → Rust)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    #[default]
    Substring,
    Glob,
    Regex,
}

#[derive(Debug, Deserialize)]
pub struct SearchSpec {
    pub query: String,
    #[serde(default = "default_true")]
    pub search_titles: bool,
    #[serde(default = "default_true")]
    pub search_urls: bool,
    #[serde(default)]
    pub mode: SearchMode,
    #[serde(default)]
    pub filter: FilterSpec,
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Structured filter (v0.0.5): applied by both `search` and `get_folder_items`
// ---------------------------------------------------------------------------

/// Combined filter applied during browsing or searching.
///
/// All fields are optional.  An axis is "active" when it is `Some(...)`
/// and the contained constraint is non-empty.  Multiple active axes
/// compose with logical AND.
#[derive(Debug, Deserialize, Default)]
pub struct FilterSpec {
    /// If set, only items whose `kind` is in this list survive.
    /// `None` (the default) lets every kind through.
    #[serde(default)]
    pub kinds: Option<Vec<ItemKind>>,

    /// Filter by `add_date` (Unix seconds).
    #[serde(default)]
    pub date_range: Option<DateRange>,

    /// Allowlist of registered domains (eTLD+1).  Case-insensitive match
    /// against `FolderItem::domain`.  Folders / separators are unaffected.
    #[serde(default)]
    pub domains: Option<Vec<String>>,

    /// Allowlist of TLDs (e.g. `"com"`, `"org"`).  Match the last label of
    /// the registered domain.  Folders / separators are unaffected.
    #[serde(default)]
    pub tlds: Option<Vec<String>>,

    /// Allowlist of URL schemes (e.g. `"http"`, `"https"`, `"ftp"`).
    /// Folders / separators are unaffected.
    #[serde(default)]
    pub schemes: Option<Vec<String>>,

    /// Folder-depth bracket relative to the document root.  Root folders
    /// are at depth 0.  Applied to every item (including folders).
    #[serde(default)]
    pub depth: Option<DepthFilter>,
}

#[derive(Debug, Deserialize, Default)]
pub struct DateRange {
    /// Inclusive lower bound.  Items with no `add_date` are excluded if
    /// `since` is set.
    pub since: Option<i64>,
    /// Inclusive upper bound.  Items with no `add_date` are excluded if
    /// `until` is set.
    pub until: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct DepthFilter {
    /// Inclusive minimum depth (0 = root).
    pub min: Option<u32>,
    /// Inclusive maximum depth.
    pub max: Option<u32>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct SearchResults {
    pub items: Vec<FolderItem>,
    pub total: usize,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct RecoveryState {
    pub paths: Vec<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct RecoveryRestoreReport {
    #[ts(type = "Array<number>")]
    pub restored_tab_ids: Vec<TabId>,
    pub restored_paths: Vec<String>,
    pub failed_paths: Vec<String>,
}

// ---------------------------------------------------------------------------
// Sanitization
// ---------------------------------------------------------------------------

/// One contiguous run of characters in a change-preview diff (mirror of
/// [`lantern_core::sanitize::diff::DiffSpan`]).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct DiffSpan {
    pub tag: DiffTag,
    pub text: String,
}

/// Classification of one span in a diff result.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
#[serde(rename_all = "snake_case")]
pub enum DiffTag {
    Equal,
    Removed,
    Added,
}

impl From<lantern_core::sanitize::diff::DiffTag> for DiffTag {
    fn from(tag: lantern_core::sanitize::diff::DiffTag) -> Self {
        match tag {
            lantern_core::sanitize::diff::DiffTag::Equal => Self::Equal,
            lantern_core::sanitize::diff::DiffTag::Removed => Self::Removed,
            lantern_core::sanitize::diff::DiffTag::Added => Self::Added,
        }
    }
}

impl From<lantern_core::sanitize::diff::DiffSpan> for DiffSpan {
    fn from(span: lantern_core::sanitize::diff::DiffSpan) -> Self {
        Self {
            tag: span.tag.into(),
            text: span.text,
        }
    }
}

/// Serialisable representation of one proposed change, sent to the preview
/// panel.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct ChangeEntry {
    /// Index into the change set; passed back in `approvals` for
    /// `apply_changeset`.
    pub index: usize,
    #[ts(type = "number")]
    pub node_id: NodeId,
    /// "url" | "title" | "folder_name"
    pub field: String,
    pub before: String,
    pub after: String,
    /// Char-level diff spans reconstructing `before` (use for strikethrough rendering).
    pub before_spans: Vec<DiffSpan>,
    /// Char-level diff spans reconstructing `after` (use for highlight rendering).
    pub after_spans: Vec<DiffSpan>,
    pub treatment_id: String,
    pub rationale: String,
    pub destructive: bool,
    /// Initial approval state (true for non-destructive changes).
    pub approved: bool,
}

/// Return value of `run_pass`: what the UI shows in the preview panel.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct ChangeSetPreview {
    #[ts(type = "number")]
    pub changeset_id: ChangeSetId,
    pub rule_set_name: String,
    pub changes: Vec<ChangeEntry>,
}

/// Return value of `apply_changeset`.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct ApplyReport {
    pub applied_count: usize,
    pub skipped_count: usize,
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// User-facing subset of [`lantern_io::Settings`].
///
/// Runtime-managed fields (recent files, recoverable documents, session flag)
/// are intentionally omitted: the UI edits preferences, not internal state.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct AppSettings {
    pub theme: ThemeSetting,
    pub dead_link_checker_opt_in: bool,
    #[ts(type = "number")]
    pub recent_files_max: u32,
    pub crash_recovery_enabled: bool,
    /// List-row density for the bookmark list pane.
    pub list_density: ListDensitySetting,
    /// Path to the on-disk settings file (read-only; shown for the "locate
    /// settings" affordance in the UI).
    pub settings_path: String,
    /// Path to the directory holding `*.lantern-rules.toml` files.
    pub rules_dir: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
#[serde(rename_all = "snake_case")]
pub enum ListDensitySetting {
    Compact,
    Comfortable,
}

impl From<lantern_io::ListDensity> for ListDensitySetting {
    fn from(d: lantern_io::ListDensity) -> Self {
        match d {
            lantern_io::ListDensity::Compact => Self::Compact,
            lantern_io::ListDensity::Comfortable => Self::Comfortable,
        }
    }
}

impl From<ListDensitySetting> for lantern_io::ListDensity {
    fn from(d: ListDensitySetting) -> Self {
        match d {
            ListDensitySetting::Compact => Self::Compact,
            ListDensitySetting::Comfortable => Self::Comfortable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
#[serde(rename_all = "snake_case")]
pub enum ThemeSetting {
    System,
    Light,
    Dark,
}

impl From<lantern_io::Theme> for ThemeSetting {
    fn from(t: lantern_io::Theme) -> Self {
        match t {
            lantern_io::Theme::System => Self::System,
            lantern_io::Theme::Light => Self::Light,
            lantern_io::Theme::Dark => Self::Dark,
        }
    }
}

impl From<ThemeSetting> for lantern_io::Theme {
    fn from(t: ThemeSetting) -> Self {
        match t {
            ThemeSetting::System => Self::System,
            ThemeSetting::Light => Self::Light,
            ThemeSetting::Dark => Self::Dark,
        }
    }
}

// ---------------------------------------------------------------------------
// Rule sets
// ---------------------------------------------------------------------------

/// Summary for a single rule set, used by `list_rule_sets`.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct RuleSetSummary {
    pub name: String,
    /// Treatment count (for the card subtitle in the UI).
    #[ts(type = "number")]
    pub treatment_count: u32,
    /// True for the shipped built-in rule sets.
    pub is_builtin: bool,
    /// Absolute path to the `.lantern-rules.toml` file.
    pub path: String,
}

/// Full detail for one rule set (treatment list in order) used by the editor.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct RuleSetDetail {
    pub name: String,
    /// Ordered treatment IDs.  Convenience shortcut for callers that don't
    /// care about per-treatment config (most of the UI).
    pub treatment_ids: Vec<String>,
    /// Same order as `treatment_ids` but pairs each ID with its current
    /// configuration as a JSON object (e.g. `{"params": [...]}` for
    /// `url.qp.custom`).  `null` for stateless treatments.
    pub treatments: Vec<RuleSetTreatment>,
    pub is_builtin: bool,
    pub path: String,
}

/// One ordered entry in a rule-set, exposed to the UI.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct RuleSetTreatment {
    pub id: String,
    /// JSON-shaped current config (e.g. `{"params": ["fbclid"]}`).  `null`
    /// when the treatment has no parameters or when it is unconfigured.
    #[ts(type = "Record<string, unknown> | null")]
    pub config: Option<serde_json::Value>,
}

/// Treatment registry entry: metadata for the "Add treatment" picker in the UI.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct TreatmentInfo {
    pub id: String,
    pub name: String,
    /// "url_query_param" | "url_path" | "url_fragment" | "url_host" | "title" |
    /// "folder_name" | "cross_field"
    pub category: String,
    pub destructive: bool,
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ExportScope {
    /// Export the entire document.
    WholeDocument,
    /// Export only the subtree rooted at `root_id`.  v0.0.5.
    ///
    /// The exported file is a self-contained Netscape bookmark file whose
    /// `<H1>` is the source folder's title.  Source-document undo state and
    /// the original path are not propagated; the subtree starts "clean".
    Subtree { root_id: NodeId },
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct ExportReport {
    pub path: String,
    pub bytes_written: usize,
}

// ---------------------------------------------------------------------------
// Dead-link checker (v0.0.4)
// ---------------------------------------------------------------------------
//
// All checker types are gated behind the `checker` feature so the
// `--no-default-features` (offline-only, v0.0.6) build does not reference any
// of the lantern-net types (even the always-available result enums), keeping
// the binary's symbol surface free of any networking-adjacent identifiers.

/// One row of [`LinkCheckReport::entries`]: the bookmark and what came back
/// from probing it.
///
/// The `status` field mirrors [`lantern_net::LinkStatus`] using the same
/// internally-tagged JSON shape so the UI can pattern-match by `status.kind`.
#[cfg(feature = "checker")]
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct LinkCheckEntry {
    #[ts(type = "number")]
    pub node_id: NodeId,
    pub url: String,
    pub title: String,
    pub status: LinkStatusView,
    /// Time-to-status, milliseconds.  `0` for skipped URLs.
    #[ts(type = "number")]
    pub elapsed_ms: u64,
}

/// JSON-stable mirror of [`lantern_net::LinkStatus`].
///
/// We keep our own type rather than re-exporting the lantern-net one because
/// `ts_rs` cannot derive bindings for a type that lives in another crate.
#[cfg(feature = "checker")]
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LinkStatusView {
    Ok {
        #[ts(type = "number")]
        code: u16,
    },
    Redirect {
        #[ts(type = "number")]
        code: u16,
    },
    ClientError {
        #[ts(type = "number")]
        code: u16,
    },
    ServerError {
        #[ts(type = "number")]
        code: u16,
    },
    Timeout,
    NetworkError {
        detail: String,
    },
    Skipped {
        reason: SkipReasonView,
    },
}

#[cfg(feature = "checker")]
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
#[serde(rename_all = "snake_case")]
pub enum SkipReasonView {
    InvalidUrl,
    UnsupportedScheme,
    Empty,
}

#[cfg(feature = "checker")]
impl From<lantern_net::SkipReason> for SkipReasonView {
    fn from(r: lantern_net::SkipReason) -> Self {
        match r {
            lantern_net::SkipReason::InvalidUrl => Self::InvalidUrl,
            lantern_net::SkipReason::UnsupportedScheme => Self::UnsupportedScheme,
            lantern_net::SkipReason::Empty => Self::Empty,
        }
    }
}

#[cfg(feature = "checker")]
impl From<lantern_net::LinkStatus> for LinkStatusView {
    fn from(s: lantern_net::LinkStatus) -> Self {
        use lantern_net::LinkStatus as L;
        match s {
            L::Ok { code } => Self::Ok { code },
            L::Redirect { code } => Self::Redirect { code },
            L::ClientError { code } => Self::ClientError { code },
            L::ServerError { code } => Self::ServerError { code },
            L::Timeout => Self::Timeout,
            L::NetworkError { detail } => Self::NetworkError { detail },
            L::Skipped { reason } => Self::Skipped {
                reason: reason.into(),
            },
        }
    }
}

#[cfg(feature = "checker")]
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct LinkCheckReport {
    pub entries: Vec<LinkCheckEntry>,
    /// Number of bookmarks the document contained when the report was built.
    pub total_bookmarks: usize,
    /// Number of entries that actually hit the network (`total - skipped`).
    pub probed: usize,
}

// ---------------------------------------------------------------------------
// Document diff (v0.0.4)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct BookmarkSnapshotView {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct ModifiedBookmarkView {
    pub before: BookmarkSnapshotView,
    pub after: BookmarkSnapshotView,
}

/// Three-way diff between two open tabs: the result of `compare_tabs`.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct DocDiffReport {
    pub left_title: String,
    pub right_title: String,
    pub added: Vec<BookmarkSnapshotView>,
    pub removed: Vec<BookmarkSnapshotView>,
    pub modified: Vec<ModifiedBookmarkView>,
}

impl From<lantern_core::diff::BookmarkSnapshot> for BookmarkSnapshotView {
    fn from(s: lantern_core::diff::BookmarkSnapshot) -> Self {
        Self {
            url: s.url,
            title: s.title,
        }
    }
}

impl From<lantern_core::diff::ModifiedBookmark> for ModifiedBookmarkView {
    fn from(m: lantern_core::diff::ModifiedBookmark) -> Self {
        Self {
            before: m.before.into(),
            after: m.after.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Cross-document merge (v0.0.7)
// ---------------------------------------------------------------------------

/// One subtree to fold into a merge.  The UI passes tab IDs (which the
/// command resolves to source-document indices).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct MergePickRequest {
    #[ts(type = "number")]
    pub source_tab_id: TabId,
    #[ts(type = "number")]
    pub root_node_id: NodeId,
}

/// IPC mirror of [`lantern_core::model::merge::ConflictStrategy`].
///
/// The two enums are kept separate so the core stays serde-free and the IPC
/// boundary controls its own JSON shape.  Use the `From` impls below to
/// convert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
#[serde(rename_all = "snake_case")]
pub enum ConflictStrategyView {
    KeepFirst,
    KeepNewest,
    KeepBoth,
}

impl From<ConflictStrategyView> for lantern_core::model::merge::ConflictStrategy {
    fn from(v: ConflictStrategyView) -> Self {
        match v {
            ConflictStrategyView::KeepFirst => Self::KeepFirst,
            ConflictStrategyView::KeepNewest => Self::KeepNewest,
            ConflictStrategyView::KeepBoth => Self::KeepBoth,
        }
    }
}

// ---------------------------------------------------------------------------
// Settings UI completeness: Keyboard / Logs / About panes (v0.0.7)
// ---------------------------------------------------------------------------

/// One row in the keyboard-shortcuts table.  Hardcoded for v0.0.7; rebinding
/// is deferred to a later milestone (PRD §8.9 captures the canonical list).
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct ShortcutBinding {
    /// Stable identifier (e.g. `"open_file"`).  Used as the React `key` and as
    /// the future hook for user-rebinding.
    pub action_id: String,
    /// Human-readable label (e.g. `"Open file"`).
    pub label: String,
    /// Pretty-printed key combo (e.g. `"Ctrl+Shift+D"`).
    pub key_combo: String,
    /// Grouping bucket: `"File"`, `"Edit"`, `"View"`, `"Tools"`, `"App"`.
    pub category: String,
}

/// One parsed log entry: output of `get_logs`.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct LogEntry {
    /// ISO 8601 timestamp string copied verbatim from the log file.
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

/// Severity level.  `Info` is the default for unrecognised level strings so
/// malformed lines don't surface as errors to the user.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Lenient parse from the level column of a log line.  Anything we don't
    /// recognise becomes `Info`; see the doc comment on the enum.
    pub fn from_token(s: &str) -> Self {
        match s.to_ascii_uppercase().as_str() {
            "WARN" | "WARNING" => Self::Warn,
            "ERROR" | "ERR" => Self::Error,
            _ => Self::Info,
        }
    }
}

/// Build-time facts surfaced in the About pane.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct BuildInfo {
    /// Crate version from `CARGO_PKG_VERSION`.
    pub version: String,
    /// `"default"` (full build) or `"offline-only"` (no `checker` feature).
    pub build_flavor: String,
    /// Compile-time Rust version, captured by `build.rs`.
    pub rust_version: String,
    /// Reserved for future use; always `None` until a git stamp is wired in.
    pub git_commit: Option<String>,
    /// SPDX-style license string (matches the workspace `LICENSE` file).
    pub license: String,
    /// Informational reference path to the ADR index.
    pub adr_index_path: String,
    /// `true` if the binary was Authenticode-signed at build time.  Driven
    /// by the `LANTERN_SIGNED=1` environment variable read by the build
    /// script, which the CI sign-windows job sets after `signtool` succeeds.
    /// Local dev builds always report `false`.
    pub signed: bool,
}
