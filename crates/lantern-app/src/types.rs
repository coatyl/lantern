//! View models that cross the IPC boundary: flat, JSON-friendly mirrors of
//! the `lantern-core` / `lantern-io` types.
//!
//! `cargo test -p lantern-app export_bindings` writes TypeScript bindings for
//! the `#[ts(export)]` types to `ui/src/ipc/bindings/`.  The UI's own
//! `ui/src/ipc/types.ts` must be kept in step by hand.

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

/// Full folder tree of a document (merge picker).
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

/// One tree-pane row.  Unlike [`TreeNode`] it carries no descendants; the
/// tree pane fetches them with `get_tree_children` on expand.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct TreeNodeLazy {
    #[ts(type = "number")]
    pub id: NodeId,
    pub name: String,
    /// Whether the folder contains folders, i.e. gets an expand chevron.
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
    /// URL host without a leading `www.`, if the URL parses.
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
// Sorting and search (UI -> Rust)
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
// Structured filter, applied by `search` and `get_folder_items`
// ---------------------------------------------------------------------------

/// An axis is active when it is `Some` and non-empty; active axes compose
/// with AND.
#[derive(Debug, Deserialize, Default)]
pub struct FilterSpec {
    /// If set, only items whose `kind` is in this list survive.
    /// `None` (the default) lets every kind through.
    #[serde(default)]
    pub kinds: Option<Vec<ItemKind>>,

    /// Filter by `add_date` (Unix seconds).
    #[serde(default)]
    pub date_range: Option<DateRange>,

    /// Allowlist of domains, compared case-insensitively with
    /// `FolderItem::domain`.  Folders / separators are unaffected.
    #[serde(default)]
    pub domains: Option<Vec<String>>,

    /// Allowlist of TLDs (e.g. `"com"`, `"org"`), matched against the last
    /// label of the domain.  Folders / separators are unaffected.
    #[serde(default)]
    pub tlds: Option<Vec<String>>,

    /// Allowlist of URL schemes (e.g. `"http"`, `"https"`, `"ftp"`).
    /// Folders / separators are unaffected.
    #[serde(default)]
    pub schemes: Option<Vec<String>>,

    /// Depth bracket below the document root (root children are depth 0).
    /// Only applied by `search`; browsing a single folder ignores it.
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

/// One run of characters in a change-preview diff.
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

/// One proposed change, as shown on the review surface.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct ChangeEntry {
    /// Index into the change set; passed back in `approvals` for
    /// `apply_changeset`.
    pub index: usize,
    #[ts(type = "number")]
    pub node_id: NodeId,
    /// `"url"`, `"title"`, `"folder_name"`, `"node"` (deletion) or
    /// `"flag:<name>"`.
    pub field: String,
    pub before: String,
    pub after: String,
    /// Character diff spans that rebuild `before`.
    pub before_spans: Vec<DiffSpan>,
    /// Character diff spans that rebuild `after`.
    pub after_spans: Vec<DiffSpan>,
    pub treatment_id: String,
    pub rationale: String,
    pub destructive: bool,
    /// Initial approval state (true for non-destructive changes).
    pub approved: bool,
    /// Title (bookmark) or name (folder) before the change; for a deletion
    /// the only description of the node.
    pub node_title: String,
    /// The node's URL before the change (bookmarks only).
    pub node_url: Option<String>,
    /// Folder names from the top of the document down to the node's parent
    /// (the root folder itself is left out).
    pub location: Vec<String>,
}

/// Return value of `run_pass`.
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

/// The user-editable part of [`lantern_io::Settings`] plus where it lives.
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
    /// Read-only: ignored by `update_settings`.
    pub settings_path: String,
    /// Read-only: ignored by `update_settings`.
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

/// One row of `list_rule_sets`.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct RuleSetSummary {
    pub name: String,
    #[ts(type = "number")]
    pub treatment_count: u32,
    pub is_builtin: bool,
    /// Path of the `.lantern-rules.toml` file.
    pub path: String,
}

/// One rule set with its ordered treatments, for the editor.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct RuleSetDetail {
    pub name: String,
    /// Ordered treatment ids.
    pub treatment_ids: Vec<String>,
    /// The same treatments, each with its configuration.
    pub treatments: Vec<RuleSetTreatment>,
    pub is_builtin: bool,
    pub path: String,
}

/// One treatment in a rule set.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct RuleSetTreatment {
    pub id: String,
    /// Configuration as JSON (e.g. `{"params": ["fbclid"]}`); `null` for
    /// treatments without parameters.
    #[ts(type = "Record<string, unknown> | null")]
    pub config: Option<serde_json::Value>,
}

/// One entry of the "Add treatment" picker.
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
    /// Export only the folder `root_id` and everything below it.
    Subtree { root_id: NodeId },
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct ExportReport {
    pub path: String,
    pub bytes_written: usize,
}

// ---------------------------------------------------------------------------
// Dead-link checker (`checker` feature only, like everything using
// `lantern-net`)
// ---------------------------------------------------------------------------

/// One probed bookmark.
#[cfg(feature = "checker")]
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct LinkCheckEntry {
    #[ts(type = "number")]
    pub node_id: NodeId,
    pub url: String,
    pub title: String,
    pub status: LinkStatusView,
    /// Milliseconds until a status was known; `0` for skipped URLs.
    #[ts(type = "number")]
    pub elapsed_ms: u64,
}

/// Mirror of [`lantern_net::LinkStatus`], tagged by `kind`.  A local type
/// because `ts-rs` cannot derive bindings for another crate's types.
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
    pub total_bookmarks: usize,
    /// Entries that went to the network (not skipped).
    pub probed: usize,
}

// ---------------------------------------------------------------------------
// Document diff
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

/// Result of `compare_tabs`.
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
// Cross-document merge
// ---------------------------------------------------------------------------

/// One subtree to copy into a merge.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct MergePickRequest {
    #[ts(type = "number")]
    pub source_tab_id: TabId,
    #[ts(type = "number")]
    pub root_node_id: NodeId,
}

/// Mirror of [`lantern_core::model::merge::ConflictStrategy`]; the core
/// stays serde-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
#[serde(rename_all = "snake_case")]
// The shared prefix is the vocabulary: these names are the IPC values.
#[allow(clippy::enum_variant_names)]
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
// Keyboard, Logs and About panes
// ---------------------------------------------------------------------------

/// One row of the keyboard-shortcuts table.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct ShortcutBinding {
    /// Stable identifier (e.g. `"open_file"`), used as the React `key`.
    pub action_id: String,
    /// Human-readable label (e.g. `"Open file"`).
    pub label: String,
    /// Pretty-printed key combo (e.g. `"Ctrl+Shift+D"`).
    pub key_combo: String,
    /// Grouping bucket: `"File"`, `"Edit"`, `"View"`, `"Tools"`, `"App"`.
    pub category: String,
}

/// One parsed line of `get_logs`.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
pub struct LogEntry {
    /// Timestamp copied verbatim from the log file.
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

/// Log severity.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../../ui/src/ipc/bindings/")]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Parse a log line's level column; anything unrecognised is `Info`.
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
    /// `rustc -V` output captured by `build.rs`.
    pub rust_version: String,
    /// Always `None`: no git stamp is embedded yet.
    pub git_commit: Option<String>,
    /// SPDX license expression.
    pub license: String,
    pub adr_index_path: String,
    /// Whether the Windows signing job signed this binary (`LANTERN_SIGNED`
    /// at build time).  Always `false` for local builds.
    pub signed: bool,
}
