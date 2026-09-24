/**
 * IPC types: the single module the UI imports backend shapes from.
 *
 * Response types are re-exported from the ts-rs bindings generated from
 * `crates/lantern-app/src/types.rs` (`npm run bindings`). Request types the
 * backend only deserialises (sort, search, filter, export scope) have no
 * generated binding and are declared here.
 */

import type { ChangeEntry as GeneratedChangeEntry } from "./bindings/ChangeEntry";
import type { ChangeSetPreview as GeneratedChangeSetPreview } from "./bindings/ChangeSetPreview";
import type { ItemKind } from "./bindings/ItemKind";

export type TabId = number;
export type NodeId = number;
export type ChangeSetId = number;

export type { AppSettings } from "./bindings/AppSettings";
export type { ApplyReport } from "./bindings/ApplyReport";
export type { BookmarkSnapshotView } from "./bindings/BookmarkSnapshotView";
export type { BuildInfo } from "./bindings/BuildInfo";
export type { ConflictStrategyView } from "./bindings/ConflictStrategyView";
export type { DiffSpan } from "./bindings/DiffSpan";
export type { DiffTag } from "./bindings/DiffTag";
export type { DocDiffReport } from "./bindings/DocDiffReport";
export type { DocStats } from "./bindings/DocStats";
export type { ExportReport } from "./bindings/ExportReport";
export type { FolderItem } from "./bindings/FolderItem";
export type { ItemKind } from "./bindings/ItemKind";
export type { ItemPage } from "./bindings/ItemPage";
export type { LinkCheckEntry } from "./bindings/LinkCheckEntry";
export type { LinkCheckReport } from "./bindings/LinkCheckReport";
export type { LinkStatusView as LinkStatus } from "./bindings/LinkStatusView";
export type { ListDensitySetting } from "./bindings/ListDensitySetting";
export type { LogEntry } from "./bindings/LogEntry";
export type { LogLevel } from "./bindings/LogLevel";
export type { MergePickRequest } from "./bindings/MergePickRequest";
export type { ModifiedBookmarkView } from "./bindings/ModifiedBookmarkView";
export type { RecoveryRestoreReport } from "./bindings/RecoveryRestoreReport";
export type { RecoveryState } from "./bindings/RecoveryState";
export type { RuleSetDetail } from "./bindings/RuleSetDetail";
export type { RuleSetSummary } from "./bindings/RuleSetSummary";
export type { RuleSetTreatment } from "./bindings/RuleSetTreatment";
export type { SearchResults } from "./bindings/SearchResults";
export type { ShortcutBinding } from "./bindings/ShortcutBinding";
export type { SkipReasonView as SkipReason } from "./bindings/SkipReasonView";
export type { TabInfo } from "./bindings/TabInfo";
export type { ThemeSetting } from "./bindings/ThemeSetting";
export type { TreatmentInfo } from "./bindings/TreatmentInfo";
export type { TreeNode } from "./bindings/TreeNode";
export type { TreeNodeLazy } from "./bindings/TreeNodeLazy";
export type { TreeView } from "./bindings/TreeView";

// Rust declares `field` as a String; narrow it to the values the backend
// emits so the review surface can switch on it exhaustively.
export type ChangeEntry = Omit<GeneratedChangeEntry, "field"> & {
  /** `node` is a whole-node deletion (e.g. an exact-URL duplicate). */
  field: "url" | "title" | "folder_name" | "node";
};

// Same shape as the binding, carrying the narrowed ChangeEntry.
export type ChangeSetPreview = Omit<GeneratedChangeSetPreview, "changes"> & {
  changes: ChangeEntry[];
};

// ---------------------------------------------------------------------------
// Request types (UI → Rust)
// ---------------------------------------------------------------------------

export type SortColumn = "title" | "url" | "domain" | "add_date" | "last_modified";

export interface SortSpec {
  column: SortColumn;
  descending: boolean;
}

export type SearchMode = "substring" | "glob" | "regex";

export interface SearchSpec {
  query: string;
  search_titles: boolean;
  search_urls: boolean;
  mode: SearchMode;
  filter?: FilterSpec;
}

/**
 * Structured filter applied by `search` and `get_folder_items`. An axis is
 * active when present and non-empty; active axes compose with AND.
 */
export interface FilterSpec {
  /** Allowlist of item kinds. */
  kinds?: ItemKind[] | null;
  /** `add_date` bounds (Unix seconds); undated items drop out once set. */
  date_range?: DateRange | null;
  /** Registered domains (eTLD+1, case-insensitive). Bookmarks only. */
  domains?: string[] | null;
  /** TLDs ("com", "org"). Bookmarks only. */
  tlds?: string[] | null;
  /** URL schemes ("http", "https", "ftp"). Bookmarks only. */
  schemes?: string[] | null;
  /** Folder-depth bracket (0 = root). Honoured during search only. */
  depth?: DepthFilter | null;
}

/** Inclusive bounds, Unix seconds. */
export interface DateRange {
  since?: number | null;
  until?: number | null;
}

/** Inclusive depth bounds (0 = root). */
export interface DepthFilter {
  min?: number | null;
  max?: number | null;
}

export type ExportScope =
  | { kind: "whole_document" }
  | { kind: "subtree"; root_id: NodeId };
