/**
 * IPC type definitions: mirror of crates/lantern-app/src/types.rs.
 *
 * Keep in sync with the Rust types until ts-rs codegen is wired into CI
 * (planned for v0.0.2).
 */

export type TabId = number;
export type NodeId = number;
export type ChangeSetId = number;

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

export interface DocStats {
  bookmark_count: number;
  folder_count: number;
  separator_count: number;
}

export interface TabInfo {
  id: TabId;
  title: string;
  path: string | null;
  dirty: boolean;
  stats: DocStats;
}

// ---------------------------------------------------------------------------
// Tree pane
// ---------------------------------------------------------------------------

export interface TreeNode {
  id: NodeId;
  name: string;
  children: TreeNode[];
}

export interface TreeView {
  root: TreeNode;
}

/**
 * Single tree-pane row used by the v0.0.8 progressive tree expansion API.
 *
 * Mirrors `crates/lantern-app/src/types.rs :: TreeNodeLazy`.  Unlike
 * `TreeNode`, this row does not carry its descendants; the UI fetches
 * them lazily via `ipc.getTreeChildren(tabId, parentId)` when the user
 * expands the row.  `has_children` tells the UI whether to render an
 * expand chevron.
 */
export interface TreeNodeLazy {
  id: NodeId;
  name: string;
  has_children: boolean;
}

// ---------------------------------------------------------------------------
// List pane
// ---------------------------------------------------------------------------

export type ItemKind = "bookmark" | "folder" | "separator";

export interface FolderItem {
  id: NodeId;
  kind: ItemKind;
  title: string;
  url: string | null;
  domain: string | null;
  add_date: number | null;
  last_modified: number | null;
}

export interface ItemPage {
  items: FolderItem[];
  total: number;
}

export type SortColumn =
  | "title"
  | "url"
  | "domain"
  | "add_date"
  | "last_modified";

export interface SortSpec {
  column: SortColumn;
  descending: boolean;
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

export type SearchMode = "substring" | "glob" | "regex";

export interface SearchSpec {
  query: string;
  search_titles: boolean;
  search_urls: boolean;
  mode: SearchMode;
  /** Structured filter (v0.0.5). Optional; omit for no filtering. */
  filter?: FilterSpec;
}

export interface SearchResults {
  items: FolderItem[];
  total: number;
}

// ---------------------------------------------------------------------------
// Structured filter (v0.0.5)
//   Mirror of `crates/lantern-app/src/types.rs` :: FilterSpec.
//   All axes are optional; an axis is "active" when present and non-empty.
//   Multiple active axes compose with AND.
// ---------------------------------------------------------------------------

export interface FilterSpec {
  /** Allowlist of item kinds. Omit / null = all kinds. */
  kinds?: ItemKind[] | null;
  /** Filter by `add_date` (Unix seconds). Items without a date are excluded once set. */
  date_range?: DateRange | null;
  /** Allowlist of registered domains (eTLD+1, case-insensitive). Bookmark-only. */
  domains?: string[] | null;
  /** Allowlist of TLDs ("com", "org", or ".com"). Bookmark-only. */
  tlds?: string[] | null;
  /** Allowlist of URL schemes ("http", "https", "ftp"). Bookmark-only. */
  schemes?: string[] | null;
  /** Folder-depth bracket. Honored during search; ignored in browse mode. */
  depth?: DepthFilter | null;
}

export interface DateRange {
  /** Inclusive lower bound, Unix seconds. */
  since?: number | null;
  /** Inclusive upper bound, Unix seconds. */
  until?: number | null;
}

export interface DepthFilter {
  /** Inclusive minimum depth (0 = root). */
  min?: number | null;
  /** Inclusive maximum depth. */
  max?: number | null;
}

export interface RecoveryState {
  paths: string[];
}

export interface RecoveryRestoreReport {
  restored_tab_ids: TabId[];
  restored_paths: string[];
  failed_paths: string[];
}

// ---------------------------------------------------------------------------
// Sanitization
// ---------------------------------------------------------------------------

// Char-level diff segment: one per span between the before/after text.
// `tag` distinguishes unchanged runs from added/removed runs so the UI can
// style them differently (strikethrough red vs highlighted green).
export type DiffTag = "equal" | "removed" | "added";

export interface DiffSpan {
  tag: DiffTag;
  text: string;
}

export interface ChangeEntry {
  index: number;
  node_id: NodeId;
  field: "url" | "title" | "folder_name";
  before: string;
  after: string;
  /** Char-level diff of `before`; empty when diff was skipped (input too long). */
  before_spans: DiffSpan[];
  /** Char-level diff of `after`; empty when diff was skipped. */
  after_spans: DiffSpan[];
  treatment_id: string;
  rationale: string;
  destructive: boolean;
  approved: boolean;
}

export interface ChangeSetPreview {
  changeset_id: ChangeSetId;
  rule_set_name: string;
  changes: ChangeEntry[];
}

export interface ApplyReport {
  applied_count: number;
  skipped_count: number;
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

/**
 * Export selector: discriminated union mirroring the Rust `ExportScope`.
 *
 * - `whole_document`: write the document as-is.
 * - `subtree`:        write only the folder rooted at `root_id` (v0.0.5).
 */
export type ExportScope =
  | { kind: "whole_document" }
  | { kind: "subtree"; root_id: NodeId };

export interface ExportReport {
  path: string;
  bytes_written: number;
}

// ---------------------------------------------------------------------------
// Dead-link checker (v0.0.4)
// ---------------------------------------------------------------------------

export type SkipReason = "invalid_url" | "unsupported_scheme" | "empty";

/**
 * Internally-tagged mirror of `lantern_net::LinkStatus`.
 *
 * Pattern-match on `kind`; the payload depends on the variant.
 */
export type LinkStatus =
  | { kind: "ok"; code: number }
  | { kind: "redirect"; code: number }
  | { kind: "client_error"; code: number }
  | { kind: "server_error"; code: number }
  | { kind: "timeout" }
  | { kind: "network_error"; detail: string }
  | { kind: "skipped"; reason: SkipReason };

export interface LinkCheckEntry {
  node_id: number;
  url: string;
  title: string;
  status: LinkStatus;
  elapsed_ms: number;
}

export interface LinkCheckReport {
  entries: LinkCheckEntry[];
  total_bookmarks: number;
  probed: number;
}

// ---------------------------------------------------------------------------
// Document diff (v0.0.4)
// ---------------------------------------------------------------------------

export interface BookmarkSnapshotView {
  url: string;
  title: string;
}

export interface ModifiedBookmarkView {
  before: BookmarkSnapshotView;
  after: BookmarkSnapshotView;
}

export interface DocDiffReport {
  left_title: string;
  right_title: string;
  added: BookmarkSnapshotView[];
  removed: BookmarkSnapshotView[];
  modified: ModifiedBookmarkView[];
}

// ---------------------------------------------------------------------------
// Cross-document merge (v0.0.7, ADR-0009)
// ---------------------------------------------------------------------------

/** How to resolve URL collisions among bookmarks landing under the same parent. */
export type ConflictStrategyView = "keep_first" | "keep_newest" | "keep_both";

/** One subtree to fold into a cross-document merge. */
export interface MergePickRequest {
  source_tab_id: TabId;
  root_node_id: NodeId;
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

export type UiErrorKind =
  | "io"
  | "parse"
  | "tab_not_found"
  | "change_set_not_found"
  | "invalid_operation"
  | "internal";

export interface UiError {
  kind: UiErrorKind;
  detail: string | { line: number; column: number; reason: string };
}

// ---------------------------------------------------------------------------
// Settings (v0.0.2, persisted to %APPDATA%\Lantern\settings.toml)
// ---------------------------------------------------------------------------

export type ThemeSetting = "system" | "light" | "dark";

export type ListDensitySetting = "compact" | "comfortable";

export interface AppSettings {
  theme: ThemeSetting;
  dead_link_checker_opt_in: boolean;
  recent_files_max: number;
  crash_recovery_enabled: boolean;
  /** List-row density for the bookmark list pane. */
  list_density: ListDensitySetting;
  /** Absolute path of the settings file, for display in the UI. */
  settings_path: string;
  /** Absolute path of the rules directory, for display in the UI. */
  rules_dir: string;
}

// ---------------------------------------------------------------------------
// Rule-set editor (v0.0.2)
// ---------------------------------------------------------------------------

export interface RuleSetSummary {
  name: string;
  treatment_count: number;
  /** True for the 3 seeded rule sets (Minimal clean / Aggressive scrub / Full scrub). */
  is_builtin: boolean;
  /** Absolute path to the .lantern-rules.toml file (always set; built-ins are seeded on startup). */
  path: string;
}

export interface RuleSetTreatment {
  id: string;
  /** JSON-shaped current config (`null` for stateless treatments). */
  config: Record<string, unknown> | null;
}

export interface RuleSetDetail {
  name: string;
  treatment_ids: string[];
  treatments: RuleSetTreatment[];
  is_builtin: boolean;
  path: string;
}

/**
 * Treatment category string: one of the `category_id()` values from the
 * Rust backend.  Kept as a widened string since categories may grow in
 * future versions without breaking the UI.
 */
export type TreatmentCategory = string;

export interface TreatmentInfo {
  id: string;
  name: string;
  category: TreatmentCategory;
  destructive: boolean;
}

// ---------------------------------------------------------------------------
// Settings UI completeness: Keyboard / Logs / About panes (v0.0.7)
// ---------------------------------------------------------------------------

/** One row in the keyboard-shortcuts table. */
export interface ShortcutBinding {
  /** Stable identifier (e.g. "open_file"). */
  action_id: string;
  /** Human-readable label (e.g. "Open file"). */
  label: string;
  /** Pretty-printed key combo (e.g. "Ctrl+Shift+D"). */
  key_combo: string;
  /** Grouping bucket: "File" / "Edit" / "View" / "Tools" / "App". */
  category: string;
}

/** Severity level (`serde(rename_all = "snake_case")` on the Rust side). */
export type LogLevel = "info" | "warn" | "error";

/** One parsed log entry: output of `get_logs`. */
export interface LogEntry {
  timestamp: string;
  level: LogLevel;
  message: string;
}

/** Build-time facts surfaced in the About pane. */
export interface BuildInfo {
  version: string;
  /** "default" (full build) or "offline-only" (no `checker` feature). */
  build_flavor: string;
  /** Rust toolchain version captured at compile time. */
  rust_version: string;
  /** Reserved: `null` until a git stamp is wired in. */
  git_commit: string | null;
  /** SPDX-style license string. */
  license: string;
  /** Informational reference path to the ADR index. */
  adr_index_path: string;
  /**
   * `true` if the binary was Authenticode-signed at build time.  Driven by
   * the `LANTERN_SIGNED` env var picked up by `build.rs`; the CI
   * sign-windows job sets it after `signtool` returns 0.  Local dev
   * builds always report `false`.
   */
  signed: boolean;
}
