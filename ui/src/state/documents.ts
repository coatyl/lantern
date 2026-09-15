/**
 * Zustand store for open documents / tabs.
 *
 * This store caches what the UI currently needs; it is not the authoritative
 * model (which lives in Rust). Mutation always goes through IPC; the store
 * is refreshed by refetching after each command.
 */

import { create } from "zustand";
import type {
  TabId,
  TabInfo,
  TreeView,
  ItemPage,
  SortSpec,
  SearchResults,
  SearchMode,
  FolderItem,
  FilterSpec,
} from "../ipc/types";
import { ipc } from "../ipc";

/** True when at least one filter axis is set and non-empty. */
export function isFilterActive(filter: FilterSpec | null): boolean {
  if (!filter) return false;
  return (
    (filter.kinds?.length ?? 0) > 0 ||
    filter.date_range != null ||
    (filter.domains?.length ?? 0) > 0 ||
    (filter.tlds?.length ?? 0) > 0 ||
    (filter.schemes?.length ?? 0) > 0 ||
    filter.depth != null
  );
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface FolderCrumb {
  id: number;
  name: string;
}

/** Default page size for the list pane. */
export const PAGE_SIZE = 200;

function sameCrumbs(a: FolderCrumb[], b: FolderCrumb[]): boolean {
  if (a.length !== b.length) return false;
  return a.every((crumb, index) => crumb.id === b[index]?.id && crumb.name === b[index]?.name);
}

async function fetchFolderPage(
  activeTab: TabId,
  sortSpec: SortSpec,
  pageSize: number,
  crumbs: FolderCrumb[],
  filter: FilterSpec | null,
): Promise<ItemPage> {
  const folderId = crumbs.at(-1)?.id ?? 0;
  return ipc.getFolderItems(
    activeTab,
    folderId,
    sortSpec,
    0,
    pageSize,
    filter ?? undefined,
  );
}

interface DocumentsState {
  // ── Open tabs ─────────────────────────────────────────────────────────────
  tabs: TabInfo[];
  activeTab: TabId | null;

  // ── Tree / list panes ────────────────────────────────────────────────────
  /**
   * Legacy eager tree: kept null in v0.0.8 because `TreePane` now uses
   * the progressive `getTreeRoot` / `getTreeChildren` IPC commands and no
   * longer holds the whole tree in the store.  Retained for backwards
   * compatibility with any consumer still typed against `TreeView`.
   */
  tree: TreeView | null;
  /**
   * Monotonic counter bumped each time `refreshTree` is called.  The
   * `TreePane` watches this value (in addition to the active tab id) so
   * structural edits like rename / delete / apply that fire `refreshTree`
   * still cause the lazy tree to refetch its root and clear its
   * children cache.  v0.0.8 progressive tree expansion.
   */
  treeVersion: number;
  listPage: ItemPage | null;

  /**
   * Navigation trail.  The last entry is the currently displayed folder.
   * An empty stack means the document root (id = 0).
   */
  folderStack: FolderCrumb[];
  backHistory: FolderCrumb[][];
  forwardHistory: FolderCrumb[][];

  sortSpec: SortSpec;

  // ── Pagination ────────────────────────────────────────────────────────────
  /** Current page number (1-based). Resets to 1 on folder navigation. */
  page: number;
  pageSize: number;

  // ── Selection ────────────────────────────────────────────────────────────
  selectedItem: FolderItem | null;

  // ── Pending structural edit (delete confirmation) ─────────────────────────
  pendingDelete: FolderItem | null;

  // ── Search ───────────────────────────────────────────────────────────────
  isSearchMode: boolean;
  searchResults: SearchResults | null;

  // ── Structured filter (v0.0.5) ────────────────────────────────────────────
  /** Active filter applied to both browsing and searching. `null` = no filter. */
  filter: FilterSpec | null;

  // ── Derived helpers (not stored; computed from folderStack) ──────────────
  /** ID of the currently displayed folder (0 = document root). */
  activeFolderId: () => number;

  // ── Actions ───────────────────────────────────────────────────────────────
  setActiveTab: (id: TabId) => Promise<void>;
  refreshTabs: () => Promise<void>;
  refreshTree: () => Promise<void>;

  /**
   * Navigate to a folder and push it onto the breadcrumb stack.
   * Called when the user double-clicks a folder in the list pane.
   */
  navigateTo: (id: number, name: string) => Promise<void>;

  /**
   * Navigate directly to a specific crumb index (0 = first crumb after root).
   * Truncates the stack back to that level.
   * Pass -1 to go to the document root.
   */
  navigateToAncestor: (index: number) => Promise<void>;

  /**
   * Jump directly to any folder from the tree pane.
   * Accepts the full crumb path from root so the breadcrumb shows all ancestors.
   * Pass an empty array to go to the document root.
   */
  jumpToFolder: (crumbs: FolderCrumb[]) => Promise<void>;
  goBack: () => Promise<void>;
  goForward: () => Promise<void>;

  openFile: (path: string) => Promise<void>;
  closeTab: (id: TabId) => Promise<void>;
  setSelectedItem: (item: FolderItem | null) => void;
  setPendingDelete: (item: FolderItem | null) => void;

  refreshList: () => Promise<void>;

  /** Change the current page and reload the list. */
  setPage: (page: number) => Promise<void>;

  // ── Create node actions ────────────────────────────────────────────────────
  createBookmark: (title: string, url: string) => Promise<number>;
  createFolder: (name: string) => Promise<number>;
  createSeparator: () => Promise<number>;

  runSearch: (query: string, searchTitles: boolean, searchUrls: boolean, mode?: SearchMode) => Promise<void>;
  clearSearch: () => void;

  /** Replace the active filter and refresh the current view. Pass `null` to clear. */
  setFilter: (filter: FilterSpec | null) => Promise<void>;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useDocuments = create<DocumentsState>((set, get) => ({
  // ── Initial state ─────────────────────────────────────────────────────────
  tabs: [],
  activeTab: null,
  tree: null,
  treeVersion: 0,
  listPage: null,
  folderStack: [],
  backHistory: [],
  forwardHistory: [],
  sortSpec: { column: "title", descending: false },
  page: 1,
  pageSize: PAGE_SIZE,
  selectedItem: null,
  pendingDelete: null,
  isSearchMode: false,
  searchResults: null,
  filter: null,

  activeFolderId: () => get().folderStack.at(-1)?.id ?? 0,

  // ── Tab actions ───────────────────────────────────────────────────────────

  setActiveTab: async (id) => {
    set({
      activeTab: id,
      tree: null,
      listPage: null,
      folderStack: [],
      backHistory: [],
      forwardHistory: [],
      page: 1,
      selectedItem: null,
      isSearchMode: false,
      searchResults: null,
    });
    await Promise.all([get().refreshTree(), get().refreshList()]);
  },

  refreshTabs: async () => {
    const tabs = await ipc.listTabs();
    set({ tabs });
    const { activeTab } = get();
    if (activeTab !== null && !tabs.find((t) => t.id === activeTab)) {
      set({
        activeTab: null,
        tree: null,
        listPage: null,
        folderStack: [],
        backHistory: [],
        forwardHistory: [],
        page: 1,
        selectedItem: null,
        isSearchMode: false,
        searchResults: null,
      });
    }
  },

  /**
   * Signal the lazy tree pane (and any other consumer keyed off
   * `treeVersion`) that the tree shape may have changed.  Rather than
   * eagerly refetching the whole tree (the v0.0.6 behaviour) we just bump
   * a monotonic counter; `TreePane` notices and reloads its lazy root.
   * v0.0.8 progressive tree expansion.
   */
  refreshTree: async () => {
    set((s) => ({ treeVersion: s.treeVersion + 1 }));
  },

  refreshList: async () => {
    const { activeTab, sortSpec, page, pageSize, filter } = get();
    if (activeTab === null) return;
    const folderId = get().activeFolderId();
    const offset = (page - 1) * pageSize;
    const result = await ipc.getFolderItems(
      activeTab,
      folderId,
      sortSpec,
      offset,
      pageSize,
      filter ?? undefined,
    );
    set({ listPage: result, selectedItem: null, isSearchMode: false, searchResults: null });
  },

  setPage: async (page) => {
    set({ page });
    await get().refreshList();
  },

  // ── Navigation ────────────────────────────────────────────────────────────

  navigateTo: async (id, name) => {
    const { activeTab, sortSpec, pageSize, folderStack, backHistory, filter } = get();
    if (activeTab === null) return;
    const newStack = [...folderStack, { id, name }];
    if (sameCrumbs(folderStack, newStack)) return;
    set({
      folderStack: newStack,
      backHistory: [...backHistory, folderStack],
      forwardHistory: [],
      page: 1,
      selectedItem: null,
      isSearchMode: false,
      searchResults: null,
    });
    const result = await fetchFolderPage(activeTab, sortSpec, pageSize, newStack, filter);
    set({ listPage: result });
  },

  navigateToAncestor: async (index) => {
    const { activeTab, sortSpec, pageSize, folderStack, backHistory, filter } = get();
    if (activeTab === null) return;

    const newStack = index < 0 ? [] : folderStack.slice(0, index + 1);
    if (sameCrumbs(folderStack, newStack)) return;

    set({
      folderStack: newStack,
      backHistory: [...backHistory, folderStack],
      forwardHistory: [],
      page: 1,
      selectedItem: null,
      isSearchMode: false,
      searchResults: null,
    });
    const result = await fetchFolderPage(activeTab, sortSpec, pageSize, newStack, filter);
    set({ listPage: result });
  },

  jumpToFolder: async (crumbs) => {
    const { activeTab, sortSpec, pageSize, folderStack, backHistory, filter } = get();
    if (activeTab === null) return;
    if (sameCrumbs(folderStack, crumbs)) return;
    set({
      folderStack: crumbs,
      backHistory: [...backHistory, folderStack],
      forwardHistory: [],
      page: 1,
      selectedItem: null,
      isSearchMode: false,
      searchResults: null,
    });
    const result = await fetchFolderPage(activeTab, sortSpec, pageSize, crumbs, filter);
    set({ listPage: result });
  },

  goBack: async () => {
    const {
      activeTab,
      sortSpec,
      pageSize,
      folderStack,
      backHistory,
      forwardHistory,
      filter,
    } = get();
    if (activeTab === null || backHistory.length === 0) return;

    const target = backHistory[backHistory.length - 1];
    set({
      folderStack: target,
      backHistory: backHistory.slice(0, -1),
      forwardHistory: [...forwardHistory, folderStack],
      page: 1,
      selectedItem: null,
      isSearchMode: false,
      searchResults: null,
    });
    const result = await fetchFolderPage(activeTab, sortSpec, pageSize, target, filter);
    set({ listPage: result });
  },

  goForward: async () => {
    const {
      activeTab,
      sortSpec,
      pageSize,
      folderStack,
      backHistory,
      forwardHistory,
      filter,
    } = get();
    if (activeTab === null || forwardHistory.length === 0) return;

    const target = forwardHistory[forwardHistory.length - 1];
    set({
      folderStack: target,
      backHistory: [...backHistory, folderStack],
      forwardHistory: forwardHistory.slice(0, -1),
      page: 1,
      selectedItem: null,
      isSearchMode: false,
      searchResults: null,
    });
    const result = await fetchFolderPage(activeTab, sortSpec, pageSize, target, filter);
    set({ listPage: result });
  },

  // ── File actions ──────────────────────────────────────────────────────────

  openFile: async (path) => {
    const tabId = await ipc.openFile(path);
    await get().refreshTabs();
    await get().setActiveTab(tabId);
  },

  closeTab: async (id) => {
    await ipc.closeTab(id);
    await get().refreshTabs();
  },

  setSelectedItem: (item) => set({ selectedItem: item }),
  setPendingDelete: (item) => set({ pendingDelete: item }),

  // ── Create node ───────────────────────────────────────────────────────────

  createBookmark: async (title, url) => {
    const { activeTab } = get();
    if (activeTab === null) throw new Error("no active tab");
    const folderId = get().activeFolderId();
    const newId = await ipc.createBookmark(activeTab, folderId, title, url);
    await Promise.all([get().refreshTree(), get().refreshList()]);
    return newId;
  },

  createFolder: async (name) => {
    const { activeTab } = get();
    if (activeTab === null) throw new Error("no active tab");
    const folderId = get().activeFolderId();
    const newId = await ipc.createFolder(activeTab, folderId, name);
    await Promise.all([get().refreshTree(), get().refreshList()]);
    return newId;
  },

  createSeparator: async () => {
    const { activeTab } = get();
    if (activeTab === null) throw new Error("no active tab");
    const folderId = get().activeFolderId();
    const newId = await ipc.createSeparator(activeTab, folderId);
    await get().refreshList();
    return newId;
  },

  // ── Search ────────────────────────────────────────────────────────────────

  runSearch: async (query, searchTitles, searchUrls, mode = "substring") => {
    const { activeTab, filter } = get();
    if (activeTab === null) return;
    const results = await ipc.search(activeTab, {
      query,
      search_titles: searchTitles,
      search_urls: searchUrls,
      mode,
      filter: filter ?? undefined,
    });
    set({ isSearchMode: true, searchResults: results, selectedItem: null });
  },

  clearSearch: () => {
    set({ isSearchMode: false, searchResults: null, selectedItem: null });
  },

  // ── Structured filter (v0.0.5) ──────────────────────────────────────────────

  setFilter: async (filter) => {
    set({ filter, page: 1, selectedItem: null });
    const { activeTab, isSearchMode } = get();
    if (activeTab === null) return;
    if (isSearchMode) {
      // Re-run the active search with the new filter; no easy way to recover
      // the search query / mode from the store today, so we just clear; the
      // user re-issues the search.  TODO(v0.0.5): cache last search args.
      get().clearSearch();
      await get().refreshList();
    } else {
      await get().refreshList();
    }
  },
}));
