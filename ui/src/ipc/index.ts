/**
 * Typed IPC wrappers around @tauri-apps/api/core invoke().
 *
 * All communication between the React UI and the Rust backend goes through
 * this module. Nothing in the UI calls `invoke` directly.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  ApplyReport,
  BuildInfo,
  ChangeSetPreview,
  ConflictStrategyView,
  DocDiffReport,
  ExportReport,
  ExportScope,
  FilterSpec,
  ItemPage,
  LinkCheckReport,
  LogEntry,
  MergePickRequest,
  RecoveryRestoreReport,
  RecoveryState,
  RuleSetDetail,
  RuleSetSummary,
  RuleSetTreatment,
  SearchResults,
  SearchSpec,
  ShortcutBinding,
  SortSpec,
  TabId,
  NodeId,
  TabInfo,
  TreatmentInfo,
  TreeNodeLazy,
  TreeView,
  ChangeSetId,
} from "./types";

export const ipc = {
  // -------------------------------------------------------------------------
  // Tab management
  // -------------------------------------------------------------------------

  /** Open a bookmark file and return its new tab ID. */
  openFile: (path: string) => invoke<TabId>("open_file", { path }),

  /** Close a tab and discard its document. */
  closeTab: (tab: TabId) => invoke<void>("close_tab", { tab }),

  /** Return info for all currently open tabs. */
  listTabs: () => invoke<TabInfo[]>("list_tabs"),

  // -------------------------------------------------------------------------
  // Tree / list panes
  // -------------------------------------------------------------------------

  /** Return the full folder tree for a tab (tree pane). */
  getTree: (tab: TabId) => invoke<TreeView>("get_tree", { tab }),

  /**
   * v0.0.8 progressive tree expansion: fetch only the immediate folder
   * children of the document root.  Pair with `getTreeChildren` to load
   * deeper levels lazily as the user expands rows.
   */
  getTreeRoot: (tabId: TabId) =>
    invoke<TreeNodeLazy[]>("get_tree_root", { tabId }),

  /**
   * v0.0.8 progressive tree expansion: fetch the immediate folder
   * children of `parentId`.  Errors if the tab or node is unknown.
   */
  getTreeChildren: (tabId: TabId, parentId: number) =>
    invoke<TreeNodeLazy[]>("get_tree_children", { tabId, parentId }),

  /**
   * Return the items inside a folder for the list pane.
   *
   * Pass `folderId = 0` to get root-level items.
   * Pass `offset` / `limit` for pagination; omit both to fetch all items.
   */
  getFolderItems: (
    tab: TabId,
    folderId: number,
    sort?: SortSpec,
    offset?: number,
    limit?: number,
    filter?: FilterSpec,
  ) =>
    invoke<ItemPage>("get_folder_items", {
      tab,
      folderId,
      sort: sort ?? null,
      offset: offset ?? null,
      limit: limit ?? null,
      filter: filter ?? null,
    }),

  // -------------------------------------------------------------------------
  // Search
  // -------------------------------------------------------------------------

  search: (tab: TabId, query: SearchSpec) =>
    invoke<SearchResults>("search", { tab, query }),

  // -------------------------------------------------------------------------
  // Sanitization
  // -------------------------------------------------------------------------

  /**
   * Run a named rule set over the document and return a change set preview.
   *
   * The returned `changeset_id` must be passed to `applyChangeset` or simply
   * discarded (it expires when the tab closes).
   */
  /** `scope` limits the pass to one folder's subtree; omit for the whole document. */
  runPass: (tab: TabId, ruleSetName: string, scope?: NodeId | null) =>
    invoke<ChangeSetPreview>("run_pass", { tab, ruleSetName, scope: scope ?? null }),

  /**
   * Apply approved changes from a pending change set.
   *
   * `approvals` is a parallel boolean array matching `ChangeSetPreview.changes`.
   */
  applyChangeset: (tab: TabId, changesetId: ChangeSetId, approvals: boolean[]) =>
    invoke<ApplyReport>("apply_changeset", { tab, changesetId, approvals }),

  // -------------------------------------------------------------------------
  // Undo / redo
  // -------------------------------------------------------------------------

  undo: (tab: TabId) => invoke<void>("undo", { tab }),
  redo: (tab: TabId) => invoke<void>("redo", { tab }),

  // -------------------------------------------------------------------------
  // Export
  // -------------------------------------------------------------------------

  export: (tab: TabId, scope: ExportScope, path: string) =>
    invoke<ExportReport>("export", { tab, scope, path }),

  // -------------------------------------------------------------------------
  // Structural edits
  // -------------------------------------------------------------------------

  /** Rename a node's title (bookmark) or name (folder). Undoable. */
  renameNode: (tab: TabId, nodeId: number, newName: string) =>
    invoke<void>("rename_node", { tab, nodeId, newName }),

  /** Delete a node from the tree. Not undoable in v0.2. */
  deleteNode: (tab: TabId, nodeId: number) =>
    invoke<void>("delete_node", { tab, nodeId }),

  /** Create a new bookmark inside a folder (0 = root). Returns new node ID. */
  createBookmark: (tab: TabId, folderId: number, title: string, url: string) =>
    invoke<number>("create_bookmark", { tab, folderId, title, url }),

  /** Create a new folder inside a folder (0 = root). Returns new node ID. */
  createFolder: (tab: TabId, folderId: number, name: string) =>
    invoke<number>("create_folder", { tab, folderId, name }),

  /** Create a new separator inside a folder (0 = root). Returns new node ID. */
  createSeparator: (tab: TabId, folderId: number) =>
    invoke<number>("create_separator", { tab, folderId }),

  /**
   * Move `nodeId` to `newIndex` within `newParentId` (0 = root).
   * Used by drag-and-drop reorder.
   */
  moveNode: (tab: TabId, nodeId: number, newParentId: number, newIndex: number) =>
    invoke<void>("move_node", { tab, nodeId, newParentId, newIndex }),

  // -------------------------------------------------------------------------
  // Recent files
  // -------------------------------------------------------------------------

  /** Returns recently opened file paths, most-recent first. */
  listRecentFiles: () => invoke<string[]>("list_recent_files"),

  /** Clears the persisted recent-files list. */
  clearRecentFiles: () => invoke<void>("clear_recent_files"),

  /** Returns any recoverable document paths from the previous unclean session. */
  getRecoveryState: () => invoke<RecoveryState>("get_recovery_state"),

  /** Restores recoverable documents into new tabs. */
  restoreRecoverySession: () =>
    invoke<RecoveryRestoreReport>("restore_recovery_session"),

  /** Dismiss the current recovery prompt without restoring files. */
  dismissRecoverySession: () => invoke<void>("dismiss_recovery_session"),

  // -------------------------------------------------------------------------
  // Settings (v0.0.2)
  // -------------------------------------------------------------------------

  /** Read the persisted app settings from disk. */
  getSettings: () => invoke<AppSettings>("get_settings"),

  /** Write the app settings back to disk atomically. */
  updateSettings: (settings: AppSettings) =>
    invoke<void>("update_settings", { settings }),

  // -------------------------------------------------------------------------
  // Rule-set editor (v0.0.2)
  // -------------------------------------------------------------------------

  /**
   * List rule sets available on disk.  Built-ins are always included, even
   * if the user hasn't explicitly saved them, so the picker never comes up
   * empty on a fresh install.
   */
  listRuleSets: () => invoke<RuleSetSummary[]>("list_rule_sets"),

  /**
   * Fetch the treatment IDs that make up a rule set.  Falls back to the
   * built-in catalogue if the on-disk file is missing.
   */
  getRuleSet: (name: string) =>
    invoke<RuleSetDetail>("get_rule_set", { name }),

  /** Persist (create or overwrite) a rule set.  Fails for built-in names. */
  saveRuleSet: (name: string, treatmentIds: string[]) =>
    invoke<void>("save_rule_set", { name, treatmentIds }),

  /**
   * Persist a rule set with per-treatment config blocks (custom QP list,
   * regex pattern/replacement, …).  Used by the editor when at least one
   * parameterised treatment is present.
   */
  saveRuleSetWithConfigs: (name: string, treatments: RuleSetTreatment[]) =>
    invoke<RuleSetDetail>("save_rule_set_with_configs", { name, treatments }),

  /** Delete a user-defined rule set.  Refuses to delete built-ins. */
  deleteRuleSet: (name: string) =>
    invoke<void>("delete_rule_set", { name }),

  /** Duplicate a rule set (built-in or user-defined) under a new name. */
  duplicateRuleSet: (source: string, newName: string) =>
    invoke<void>("duplicate_rule_set", { source, newName }),

  /** Return metadata (id, name, category, destructive flag) for every treatment. */
  listTreatments: () => invoke<TreatmentInfo[]>("list_treatments"),

  // -------------------------------------------------------------------------
  // Dead-link checker (v0.0.4)
  // -------------------------------------------------------------------------

  // -------------------------------------------------------------------------
  // Document diff (v0.0.4)
  // -------------------------------------------------------------------------

  /**
   * Compare two open tabs by URL identity.
   *
   * Returns added / removed / modified bookmark buckets; see `DocDiffReport`.
   * Folder structure is ignored; URL strings are the equality key.
   */
  compareTabs: (left: TabId, right: TabId) =>
    invoke<DocDiffReport>("compare_tabs", { left, right }),

  /**
   * Probe every bookmark in `tab` for reachability.
   *
   * Refused by the backend if the user has not opted in via Settings →
   * "Dead-link checker"; the resulting `UiError` has kind `invalid_operation`.
   * The probe is `Promise`-based and may take many seconds for a large
   * document; show progress feedback while it runs.
   */
  checkDeadLinks: (tab: TabId) =>
    invoke<LinkCheckReport>("check_dead_links", { tab }),

  // -------------------------------------------------------------------------
  // Cross-document merge (v0.0.7, ADR-0009)
  // -------------------------------------------------------------------------

  /**
   * Build a brand-new document from picked subtrees of one or more open tabs
   * and register it as a new tab.  Source documents are read-only inputs;
   * see ADR-0009 for the full contract.
   *
   * Returns the new tab's ID.
   */
  mergeDocuments: (
    picks: MergePickRequest[],
    strategy: ConflictStrategyView,
    mergedRootName: string,
  ) =>
    invoke<TabId>("merge_documents", {
      picks,
      strategy,
      mergedRootName,
    }),

  // -------------------------------------------------------------------------
  // Settings UI completeness (v0.0.7): Keyboard / Logs / About panes
  // -------------------------------------------------------------------------

  /** Static list of keyboard shortcuts (PRD §8.9).  Hardcoded for v0.0.7. */
  listShortcuts: (): Promise<ShortcutBinding[]> =>
    invoke<ShortcutBinding[]>("list_shortcuts"),

  /** Last 200 entries from `<settings_dir>/logs/lantern.log`. */
  getLogs: (): Promise<LogEntry[]> => invoke<LogEntry[]>("get_logs"),

  /** Compile-time facts (version, build flavor, rust version, …). */
  getBuildInfo: (): Promise<BuildInfo> => invoke<BuildInfo>("get_build_info"),
} as const;
