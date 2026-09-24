/**
 * Centre pane: list of items in the selected folder, plus search bar.
 *
 * Features:
 * - Breadcrumb navigation (landmark)
 * - Inline search (substring + glob + regex)
 * - Kind filter drawer (Bookmarks / Folders / Separators) with `aria-expanded` /
 *   `aria-controls` on the toggle (v0.0.7 a11y P1)
 * - Inline create: New Bookmark / New Folder / New Separator (toolbar buttons)
 * - Keyboard navigation (↑↓ Enter F2 Delete Escape)
 * - Drag-and-drop reorder via @dnd-kit (browse mode only)
 * - Pagination controls (shown when folder > pageSize)
 * - **Virtualised row rendering** via `react-window`'s `FixedSizeList`
 *   (v0.0.7 perf hardening, slice 1): only the visible window of rows
 *   is mounted, so 50 k-item folders stay smooth.
 */

import {
  useState,
  useEffect,
  useRef,
  useMemo,
  memo,
  type FormEvent,
  type CSSProperties,
  type KeyboardEvent as ReactKE,
} from "react";
import { FixedSizeList, type ListChildComponentProps } from "react-window";
import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  verticalListSortingStrategy,
  useSortable,
  arrayMove,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";

import { useDocuments, isFilterActive } from "../state/documents";
import { ipc } from "../ipc";
import type { SearchMode } from "../ipc/types";
import {
  FolderIcon,
  LinkIcon,
  SeparatorIcon,
  HomeIcon,
  ChevronRightIcon,
  ChevronDownIcon,
  GripIcon,
  XIcon,
  CheckIcon,
  SearchIcon,
  PlusIcon,
} from "../components/Icons";
import { FilterDrawer } from "../components/FilterDrawer";
import { SkeletonRow } from "../components/Skeleton";
import { EmptyState, EmptyFolderIcon, EmptySearchIcon } from "../components/EmptyState";
import { useElementSize } from "../hooks/useElementSize";
import { useT } from "../i18n/I18nProvider";
import { useToast } from "../hooks/useToast";
import { SEARCH_FOCUS_EVENT } from "../components/paletteCommands";
import type { FolderItem, SortColumn } from "../ipc/types";

const COLUMNS: { key: SortColumn | null; label: string; className: string }[] = [
  { key: "title",    label: "Title",   className: "flex-1 min-w-0" },
  { key: "domain",   label: "Domain",  className: "w-36 shrink-0" },
  { key: "add_date", label: "Added",   className: "w-28 shrink-0" },
];

const SEARCH_MODES: SearchMode[] = ["substring", "glob", "regex"];

/** Delay between the last keystroke and the search request. */
const SEARCH_DEBOUNCE_MS = 180;

// ── Row-height tokens (must match `[data-density]` rules in index.css) ──
//
// Compact (default): py-1 → ~28 px tall row.
// Comfortable:       py-1 + 0.4rem extra top+bottom → ~36 px tall row.
const ROW_HEIGHT_COMPACT     = 28;
const ROW_HEIGHT_COMFORTABLE = 36;

/**
 * Reads the live `data-density` attribute off `<html>` and converts it to a
 * pixel row height.  Subscribes to attribute mutations so toggling density
 * in Settings re-measures without a remount.  Falls back to compact when
 * the attribute is missing (initial paint / older settings).
 */
function useRowHeight(): number {
  const read = () => {
    if (typeof document === "undefined") return ROW_HEIGHT_COMPACT;
    const value = document.documentElement.getAttribute("data-density");
    return value === "comfortable" ? ROW_HEIGHT_COMFORTABLE : ROW_HEIGHT_COMPACT;
  };

  const [height, setHeight] = useState<number>(read);

  useEffect(() => {
    if (typeof MutationObserver === "undefined") return;
    const observer = new MutationObserver(() => setHeight(read()));
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-density"],
    });
    return () => observer.disconnect();
  }, []);

  return height;
}

export default function ListPane() {
  const {
    listPage,
    sortSpec,
    refreshList,
    refreshTree,
    isSearchMode,
    searchResults,
    runSearch,
    clearSearch,
    activeTab,
    selectedItem,
    setSelectedItem,
    setPendingDelete,
    folderStack,
    navigateTo,
    navigateToAncestor,
    page,
    pageSize,
    setPage,
    createBookmark,
    createFolder,
    createSeparator,
    filter,
    setFilter,
  } = useDocuments();
  const t = useT();
  const { toast } = useToast();

  const [searchInput, setSearchInput]   = useState("");
  const [searchTitles, setSearchTitles] = useState(true);
  const [searchUrls, setSearchUrls]     = useState(true);
  const [searchMode, setSearchMode]     = useState<SearchMode>("substring");
  const [searching, setSearching]       = useState(false);
  const [searchError, setSearchError]   = useState<string | null>(null);
  const [newMenuOpen, setNewMenuOpen]   = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const focusSearch = () => {
      searchInputRef.current?.focus();
      searchInputRef.current?.select();
    };
    window.addEventListener(SEARCH_FOCUS_EVENT, focusSearch);
    return () => window.removeEventListener(SEARCH_FOCUS_EVENT, focusSearch);
  }, []);

  // ── Filter drawer (server-side, store-backed) ─────────────────────────────
  const [showFilter, setShowFilter] = useState(false);
  const filterActive = isFilterActive(filter);

  // ── Inline rename ─────────────────────────────────────────────────────────
  const [renamingId, setRenamingId]   = useState<number | null>(null);
  const [renameValue, setRenameValue] = useState("");

  // ── Inline create ─────────────────────────────────────────────────────────
  type CreatingKind = "bookmark" | "folder" | "separator" | null;
  const [creatingKind, setCreatingKind]   = useState<CreatingKind>(null);
  const [newTitle, setNewTitle]           = useState("");
  const [newUrl, setNewUrl]               = useState("");
  const [createBusy, setCreateBusy]       = useState(false);

  const startCreating = (kind: Exclude<CreatingKind, null>) => {
    setCreatingKind(kind);
    setNewTitle("");
    setNewUrl("");
  };

  const cancelCreate = () => setCreatingKind(null);

  const commitCreate = async () => {
    if (!activeTab || createBusy) return;
    setCreateBusy(true);
    try {
      if (creatingKind === "bookmark") {
        const t = newTitle.trim() || "New Bookmark";
        const u = newUrl.trim()   || "https://";
        await createBookmark(t, u);
      } else if (creatingKind === "folder") {
        const n = newTitle.trim() || "New Folder";
        await createFolder(n);
      } else if (creatingKind === "separator") {
        await createSeparator();
      }
      setCreatingKind(null);
    } catch {
      // creation failed; keep form open
    } finally {
      setCreateBusy(false);
    }
  };

  // ── Sort ──────────────────────────────────────────────────────────────────
  const handleSort = (col: SortColumn | null) => {
    if (!col || isSearchMode) return;
    const descending = sortSpec.column === col ? !sortSpec.descending : false;
    useDocuments.setState({ sortSpec: { column: col, descending } });
    refreshList();
  };

  // ── Search (as you type) ──────────────────────────────────────────────────
  const search = async () => {
    const query = searchInput.trim();
    if (!activeTab) return;
    if (!query) {
      if (useDocuments.getState().isSearchMode) clearSearch();
      setSearchError(null);
      return;
    }
    setSearching(true);
    try {
      await runSearch(query, searchTitles, searchUrls, searchMode);
      setSearchError(null);
    } catch (e) {
      // Typically an incomplete regex / glob while the user is still typing.
      setSearchError(e instanceof Error ? e.message : String(e));
    } finally {
      setSearching(false);
    }
  };
  const searchRef = useRef(search);
  searchRef.current = search;

  useEffect(() => {
    const handle = window.setTimeout(() => void searchRef.current(), SEARCH_DEBOUNCE_MS);
    return () => window.clearTimeout(handle);
  }, [searchInput, searchTitles, searchUrls, searchMode, activeTab]);

  const handleSearchSubmit = (e: FormEvent) => {
    e.preventDefault();
    void search();
  };

  const handleClearSearch = () => {
    clearSearch();
    setSearchInput("");
    setSearchError(null);
  };

  // Browsing to another folder ends the search.
  const folderKey = folderStack.map((c) => c.id).join("/");
  useEffect(() => {
    setSearchInput("");
    setSearchError(null);
  }, [folderKey, activeTab]);

  // ── Display items (declared early, used by keyboard handler) ──────────────
  // Note: kind filter is now server-side via the store's `filter`, so no
  // client-side filtering is needed here.
  const displayItems = isSearchMode
    ? (searchResults?.items ?? [])
    : (listPage?.items ?? []);
  const displayTotal = isSearchMode
    ? (searchResults?.total ?? 0)
    : (listPage?.total ?? 0);

  // ── Rename helpers ────────────────────────────────────────────────────────
  const startRename = (item: FolderItem) => {
    if (item.kind === "separator") return;
    setRenamingId(item.id);
    setRenameValue(item.title || "");
  };

  const commitRename = async () => {
    if (renamingId === null || !activeTab) { setRenamingId(null); return; }
    const trimmed = renameValue.trim();
    const id = renamingId;
    setRenamingId(null);
    if (!trimmed) return;
    try {
      await ipc.renameNode(activeTab, id, trimmed);
      await Promise.all([refreshTree(), refreshList()]);
    } catch (e) {
      toast(
        e instanceof Error ? e.message : "Could not rename item",
        "error",
      );
    }
  };

  const cancelRename = () => setRenamingId(null);

  // ── Virtualisation: row height + viewport size ────────────────────────────
  const rowHeight = useRowHeight();
  const { ref: viewportRef, height: viewportHeight } = useElementSize<HTMLDivElement>();

  // Imperative handle on the FixedSizeList, used by keyboard nav to call
  // scrollToItem when the focused row would otherwise scroll off-screen.
  const listImperativeRef = useRef<FixedSizeList | null>(null);

  // ── Keyboard navigation ───────────────────────────────────────────────────
  // We keep displayItems/selectedItem stable between render and the handler
  // by capturing them via ref so the listener doesn't have to rebind on
  // every keystroke.
  const navStateRef = useRef({
    items:        displayItems,
    selected:     selectedItem,
    isSearchMode,
    creatingKind,
    renamingId,
  });
  navStateRef.current = {
    items:        displayItems,
    selected:     selectedItem,
    isSearchMode,
    creatingKind,
    renamingId,
  };

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const active = document.activeElement;
      const inSearch = active && active.closest("form");
      const inDialog = active && active.closest('[role="dialog"]');
      const inRename = active && (active as HTMLElement).dataset.renameInput;
      const inCreate = active && (active as HTMLElement).dataset.createInput;
      if (inSearch || inDialog || inRename || inCreate) return;

      const {
        items,
        selected,
        isSearchMode: ism,
        creatingKind: ck,
        renamingId:   rn,
      } = navStateRef.current;

      if (e.key === "Escape") {
        e.preventDefault();
        if (ck !== null)  { cancelCreate(); return; }
        if (rn !== null)  { cancelRename(); return; }
        if (ism)          { clearSearch(); setSearchInput(""); return; }
        setSelectedItem(null);
        return;
      }

      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        if (items.length === 0) return;
        e.preventDefault();
        const currentIdx = items.findIndex((it) => it.id === selected?.id);
        let nextIdx: number;
        if (e.key === "ArrowDown") {
          nextIdx = currentIdx < items.length - 1 ? currentIdx + 1 : 0;
        } else {
          nextIdx = currentIdx > 0 ? currentIdx - 1 : items.length - 1;
        }
        setSelectedItem(items[nextIdx]);
        // Ask react-window to bring the new selection into view.  "smart"
        // alignment is a no-op when the row is already visible; only scrolls
        // when needed, which is exactly what we want for keyboard nav.
        listImperativeRef.current?.scrollToItem(nextIdx, "smart");
        return;
      }

      if (e.key === "Enter") {
        if (!selected) return;
        e.preventDefault();
        if (selected.kind === "folder" && !ism) {
          navigateTo(selected.id, selected.title || "Folder");
        }
        return;
      }

      if (e.key === "F2") {
        if (!selected || selected.kind === "separator") return;
        e.preventDefault();
        setRenamingId(selected.id);
        setRenameValue(selected.title || "");
        return;
      }

      if (e.key === "Delete") {
        if (!selected) return;
        e.preventDefault();
        setPendingDelete(selected);
        return;
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [setSelectedItem, clearSearch, navigateTo, setPendingDelete]);

  // ── Drag-and-drop ─────────────────────────────────────────────────────────
  const sensors = useSensors(useSensor(PointerSensor, {
    activationConstraint: { distance: 6 },
  }));

  const handleDragEnd = async (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id || !activeTab || isSearchMode) return;

    const oldIndex = displayItems.findIndex((it) => it.id === active.id);
    const newIndex = displayItems.findIndex((it) => it.id === over.id);
    if (oldIndex === -1 || newIndex === -1) return;

    // Optimistic UI update.
    const reordered = arrayMove(displayItems, oldIndex, newIndex);
    useDocuments.setState((s) => ({
      listPage: s.listPage ? { ...s.listPage, items: reordered } : s.listPage,
    }));

    try {
      const folderId = useDocuments.getState().activeFolderId();
      await ipc.moveNode(activeTab, Number(active.id), folderId, newIndex);
      // Full refresh to sync server order.
      await refreshList();
    } catch {
      // Revert on failure.
      await refreshList();
    }
  };

  // ── Pagination ────────────────────────────────────────────────────────────
  const totalPages = Math.ceil((listPage?.total ?? 0) / pageSize);
  const hasPrev = page > 1;
  const hasNext = page < totalPages;

  // ── itemData passed to the virtualised Row component ──────────────────────
  // Memoised so identity is stable between renders that don't actually change
  // any input; react-window forwards `itemData` to every Row by reference,
  // so unstable identity here would invalidate every memoised row.
  const itemData = useMemo<RowData>(() => ({
    items:           displayItems,
    selectedId:      selectedItem?.id ?? null,
    renamingId,
    renameValue,
    renamePlaceholder: t("rename.placeholder"),
    dragDisabled:    isSearchMode || filterActive,
    onSelect:        (item: FolderItem) => setSelectedItem(item),
    onNavigate:      (item: FolderItem) => {
      if (item.kind === "folder") navigateTo(item.id, item.title || "Folder");
    },
    onStartRename:   startRename,
    onRenameChange:  setRenameValue,
    onRenameCommit:  commitRename,
    onRenameCancel:  cancelRename,
  }), [
    displayItems,
    selectedItem?.id,
    renamingId,
    renameValue,
    isSearchMode,
    filterActive,
    setSelectedItem,
    navigateTo,
    commitRename,
    t,
  ]);

  if (!activeTab) {
    return (
      <div className="flex-1 flex items-center justify-center text-xs text-neutral-600">
        Open a file to get started.
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">

      {/* ── Toolbar: where you are · search · filter · new ─────────────── */}
      <div className="flex items-center gap-2 px-2 h-10 border-b border-neutral-800 shrink-0">
        <nav aria-label="Breadcrumb" className="flex items-center gap-0.5 min-w-0 overflow-x-auto scrollbar-none">
          <button
            onClick={() => navigateToAncestor(-1)}
            title={t("list.root")}
            aria-current={folderStack.length === 0 ? "page" : undefined}
            className={`flex items-center gap-1.5 px-1.5 py-1 rounded text-xs shrink-0 transition-colors
                        ${folderStack.length === 0
                          ? "text-neutral-100 font-medium"
                          : "text-neutral-500 hover:text-neutral-200 hover:bg-surface-2"}`}
          >
            <HomeIcon className="w-3.5 h-3.5" />
            {folderStack.length === 0 && <span>{t("list.root")}</span>}
          </button>
          {folderStack.map((crumb, i) => {
            const current = i === folderStack.length - 1;
            return (
              <span key={crumb.id} className="flex items-center gap-0.5 min-w-0">
                <ChevronRightIcon className="w-3 h-3 text-neutral-700 shrink-0" />
                <button
                  onClick={() => navigateToAncestor(i)}
                  disabled={current}
                  aria-current={current ? "page" : undefined}
                  title={crumb.name}
                  className={`px-1.5 py-1 rounded text-xs truncate max-w-[160px] transition-colors
                              ${current
                                ? "text-neutral-100 font-medium cursor-default"
                                : "text-neutral-500 hover:text-neutral-200 hover:bg-surface-2"}`}
                >
                  {crumb.name}
                </button>
              </span>
            );
          })}
        </nav>

        <form onSubmit={handleSearchSubmit} role="search" className="ml-auto relative w-72 max-w-[45%] shrink">
          <SearchIcon className="w-3.5 h-3.5 absolute left-2 top-1/2 -translate-y-1/2 text-neutral-600 pointer-events-none" />
          <input
            ref={searchInputRef}
            type="search"
            data-lantern-search
            aria-label={t("list.search")}
            placeholder={t(`list.search.placeholder.${searchMode}`)}
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key !== "Escape") return;
              e.preventDefault();
              if (searchInput) handleClearSearch();
              else searchInputRef.current?.blur();
            }}
            aria-invalid={searchError !== null}
            className={`w-full bg-surface-2 border rounded-md pl-7 pr-7 py-1 text-xs text-neutral-200
                        placeholder-neutral-600 focus:outline-none
                        ${searchError ? "border-danger/60" : "border-neutral-800 focus:border-accent/60"}
                        ${searchMode !== "substring" ? "font-mono" : ""}`}
          />
          {searchInput && (
            <button
              type="button"
              onClick={handleClearSearch}
              aria-label={t("list.search.clear")}
              className="absolute right-1.5 top-1/2 -translate-y-1/2 p-0.5 rounded text-neutral-500 hover:text-neutral-200"
            >
              {searching ? <span className="block w-3 text-center leading-none">…</span> : <XIcon className="w-3 h-3" />}
            </button>
          )}
        </form>

        <button
          type="button"
          onClick={() => setShowFilter((f) => !f)}
          aria-expanded={showFilter}
          aria-controls="filter-drawer"
          className={`relative flex items-center gap-1 px-2 py-1 rounded-md border text-xs shrink-0 transition-colors
                      ${showFilter || filterActive
                        ? "border-accent/50 text-accent bg-accent/10"
                        : "border-neutral-800 text-neutral-400 hover:text-neutral-100 hover:border-neutral-700"}`}
        >
          {t("list.filter")}
          {filterActive && <span className="w-1.5 h-1.5 rounded-full bg-accent" aria-label={t("list.filter.active")} />}
        </button>

        <NewMenu
          open={newMenuOpen}
          onOpenChange={setNewMenuOpen}
          disabled={isSearchMode}
          onPick={(kind) => {
            setNewMenuOpen(false);
            startCreating(kind);
          }}
        />
      </div>

      {/* ── Structured filter drawer ─────────────────────────────────────── */}
      {showFilter && (
        <FilterDrawer
          id="filter-drawer"
          filter={filter}
          onChange={(next) => { void setFilter(next); }}
          showDepth={isSearchMode}
        />
      )}

      {/* ── Search results bar: options live where they matter ──────────── */}
      {(isSearchMode || searchError) && (
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1 px-3 py-1.5 bg-surface-1 border-b border-neutral-800 shrink-0 text-xs">
          {searchError ? (
            <span role="alert" className="text-danger truncate">{t("list.search.invalid")}</span>
          ) : (
            <span className="text-neutral-400" aria-live="polite">
              {t("list.search.results", { n: displayTotal, query: searchInput.trim() })}
            </span>
          )}
          <div role="radiogroup" aria-label={t("list.search.mode")} className="flex rounded-md border border-neutral-800 overflow-hidden">
            {SEARCH_MODES.map((mode) => (
              <button
                key={mode}
                type="button"
                role="radio"
                aria-checked={searchMode === mode}
                onClick={() => setSearchMode(mode)}
                className={`px-2 py-0.5 transition-colors ${
                  searchMode === mode ? "bg-accent/15 text-accent" : "text-neutral-500 hover:text-neutral-200"
                }`}
              >
                {t(`list.search.mode.${mode}`)}
              </button>
            ))}
          </div>
          <div className="flex items-center gap-3 text-neutral-400">
            <label className="flex items-center gap-1 cursor-pointer select-none">
              <input type="checkbox" checked={searchTitles} onChange={(e) => setSearchTitles(e.target.checked)} />
              {t("list.search.titles")}
            </label>
            <label className="flex items-center gap-1 cursor-pointer select-none">
              <input type="checkbox" checked={searchUrls} onChange={(e) => setSearchUrls(e.target.checked)} />
              {t("list.search.urls")}
            </label>
          </div>
          <button onClick={handleClearSearch} className="ml-auto text-neutral-500 hover:text-neutral-200">
            {t("list.search.back")}
          </button>
        </div>
      )}

      {/* ── Column headers ──────────────────────────────────────────────────
          audit P2 #21: header text neutral-500 (~3.7:1) → neutral-400 (~5.5:1)
          for AA; sort buttons gain a focus-visible ring for keyboard users. */}
      {!isSearchMode && (
        <div className="flex items-center h-7 px-2 border-b border-neutral-800
                        text-xs text-neutral-400 shrink-0 select-none">
          {/* Drag-handle + icon spacer */}
          <span className="w-9 shrink-0" />
          {COLUMNS.map(({ key, label, className }) => (
            <button key={label} onClick={() => handleSort(key)}
              className={`${className} flex items-center gap-1 text-left
                          hover:text-neutral-100 transition-colors truncate
                          rounded focus:outline-none focus-visible:ring-1
                          focus-visible:ring-accent`}>
              {label}
              {key && sortSpec.column === key && (
                <span>{sortSpec.descending ? "↓" : "↑"}</span>
              )}
            </button>
          ))}
        </div>
      )}

      {/* ── Inline creation row ──────────────────────────────────────────── */}
      {creatingKind !== null && !isSearchMode && (
        <CreationRow
          kind={creatingKind}
          title={newTitle}
          url={newUrl}
          busy={createBusy}
          onTitleChange={setNewTitle}
          onUrlChange={setNewUrl}
          onCommit={commitCreate}
          onCancel={cancelCreate}
        />
      )}

      {/* ── Rows ─────────────────────────────────────────────────────────── */}
      {/*
        The virtualised viewport.  We measure this element with
        useElementSize so we can pass a concrete pixel `height` to
        FixedSizeList; react-window does not auto-fill its parent.
      */}
      <div ref={viewportRef} className="flex-1 overflow-hidden min-h-0">
        {listPage === null && !isSearchMode ? (
          // Initial folder load: render a column of skeleton rows rather
          // than a flat "Loading…" string so the layout stays stable.
          // v0.0.11 QoL slice 1.
          <div
            className="h-full overflow-hidden"
            aria-busy="true"
            aria-label={t("loading.generic")}
          >
            {Array.from({ length: 10 }).map((_, i) => (
              <SkeletonRow key={i} />
            ))}
          </div>
        ) : displayItems.length === 0 ? (
          isSearchMode ? (
            <EmptyState
              icon={<EmptySearchIcon />}
              title={t("empty.search", { query: searchInput })}
              description={t("empty.search.description")}
              ariaLabel={t("empty.search", { query: searchInput })}
            />
          ) : (
            <EmptyState
              icon={<EmptyFolderIcon />}
              title={t("empty.list")}
              description={t("empty.list.description")}
              ariaLabel={t("empty.list")}
            />
          )
        ) : viewportHeight === 0 ? (
          // First render before ResizeObserver fires: render nothing rather
          // than a 0-tall list (react-window throws on height=0 in some
          // versions).  The very next layout effect resolves the height.
          null
        ) : (
          <DndContext
            sensors={sensors}
            collisionDetection={closestCenter}
            onDragEnd={handleDragEnd}
          >
            <SortableContext
              items={displayItems.map((i) => i.id)}
              strategy={verticalListSortingStrategy}
            >
              <FixedSizeList
                ref={listImperativeRef}
                height={viewportHeight}
                width="100%"
                itemCount={displayItems.length}
                itemSize={rowHeight}
                itemData={itemData}
                itemKey={(index, data) => (data as RowData).items[index].id}
                overscanCount={8}
              >
                {VirtualRow}
              </FixedSizeList>
            </SortableContext>
          </DndContext>
        )}
      </div>

      {/* ── Footer: item count + pagination ──────────────────────────────── */}
      <div className="h-6 flex items-center justify-between px-2 border-t border-neutral-800
                      text-xs text-neutral-600 shrink-0 select-none">
        <span>
          {isSearchMode
            ? t("list.count.results", { n: displayTotal })
            : t("list.count.items", { n: displayTotal })}
        </span>
        {!isSearchMode && totalPages > 1 && (
          <div className="flex items-center gap-2">
            <button
              onClick={() => setPage(page - 1)}
              disabled={!hasPrev}
              className="text-neutral-500 hover:text-neutral-200 disabled:opacity-30 transition-colors px-1"
              title="Previous page"
            >
              ←
            </button>
            <span className="text-[10px]">{page} / {totalPages}</span>
            <button
              onClick={() => setPage(page + 1)}
              disabled={!hasNext}
              className="text-neutral-500 hover:text-neutral-200 disabled:opacity-30 transition-colors px-1"
              title="Next page"
            >
              →
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// "New" menu: bookmark / folder / separator in the current folder
// ---------------------------------------------------------------------------

type NewKind = "bookmark" | "folder" | "separator";

function NewMenu({
  open,
  onOpenChange,
  disabled,
  onPick,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  disabled: boolean;
  onPick: (kind: NewKind) => void;
}) {
  const t = useT();
  const rootRef = useRef<HTMLDivElement>(null);
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const kinds: NewKind[] = ["bookmark", "folder", "separator"];

  useEffect(() => {
    if (!open) return;
    itemRefs.current[0]?.focus();
    const onPointer = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) onOpenChange(false);
    };
    window.addEventListener("mousedown", onPointer);
    return () => window.removeEventListener("mousedown", onPointer);
  }, [open, onOpenChange]);

  const onMenuKey = (e: React.KeyboardEvent) => {
    const i = itemRefs.current.findIndex((el) => el === document.activeElement);
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const next = (i + (e.key === "ArrowDown" ? 1 : kinds.length - 1)) % kinds.length;
      itemRefs.current[next]?.focus();
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onOpenChange(false);
    }
  };

  return (
    <div ref={rootRef} className="relative shrink-0">
      <button
        type="button"
        disabled={disabled}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => onOpenChange(!open)}
        className="flex items-center gap-1 px-2 py-1 rounded-md border border-neutral-800 text-xs
                   text-neutral-300 hover:text-neutral-100 hover:border-neutral-700 disabled:opacity-40 transition-colors"
      >
        <PlusIcon className="w-3 h-3" />
        {t("list.new")}
        <ChevronDownIcon className="w-3 h-3 opacity-70" />
      </button>
      {open && (
        <div
          role="menu"
          aria-label={t("list.new")}
          onKeyDown={onMenuKey}
          className="absolute right-0 top-full mt-1 z-30 w-44 py-1 rounded-md border border-neutral-800
                     bg-surface-2 shadow-lg shadow-black/30 animate-fade-in"
        >
          {kinds.map((kind, i) => (
            <button
              key={kind}
              ref={(el) => { itemRefs.current[i] = el; }}
              role="menuitem"
              type="button"
              onClick={() => onPick(kind)}
              className="w-full text-left px-3 py-1.5 text-xs text-neutral-300 hover:bg-surface-3
                         hover:text-neutral-100 focus:bg-surface-3 focus:outline-none"
            >
              {t(`list.new.${kind}`)}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Inline creation row
// ---------------------------------------------------------------------------

function CreationRow({
  kind,
  title,
  url,
  busy,
  onTitleChange,
  onUrlChange,
  onCommit,
  onCancel,
}: {
  kind:          "bookmark" | "folder" | "separator";
  title:         string;
  url:           string;
  busy:          boolean;
  onTitleChange: (v: string) => void;
  onUrlChange:   (v: string) => void;
  onCommit:      () => void;
  onCancel:      () => void;
}) {
  const handleKey = (e: ReactKE<HTMLInputElement>) => {
    if (e.key === "Enter")  { e.preventDefault(); onCommit(); }
    if (e.key === "Escape") { e.preventDefault(); onCancel(); }
  };

  return (
    <div className="flex items-center gap-1.5 px-2 py-1.5 border-b border-accent/30
                    bg-accent/5 shrink-0">
      {/* Kind icon */}
      <span className="w-4 shrink-0 flex items-center justify-center">
        {kind === "folder" ? (
          <FolderIcon className="w-3.5 h-3.5 text-accent/70" />
        ) : kind === "separator" ? (
          <SeparatorIcon className="w-3.5 h-3.5 text-neutral-600" />
        ) : (
          <LinkIcon className="w-3.5 h-3.5 text-neutral-400" />
        )}
      </span>

      {kind === "separator" ? (
        <span className="flex-1 text-xs text-neutral-500 italic">New separator</span>
      ) : (
        <>
          <input
            autoFocus
            data-create-input
            placeholder={kind === "folder" ? "Folder name…" : "Title…"}
            value={title}
            onChange={(e) => onTitleChange(e.target.value)}
            onKeyDown={handleKey}
            disabled={busy}
            className="flex-1 min-w-0 bg-surface-2 border border-accent/40 rounded
                       px-1.5 py-0.5 text-xs text-neutral-100 placeholder-neutral-600
                       focus:outline-none focus:border-accent disabled:opacity-40"
          />
          {kind === "bookmark" && (
            <input
              data-create-input
              placeholder="URL…"
              value={url}
              onChange={(e) => onUrlChange(e.target.value)}
              onKeyDown={handleKey}
              disabled={busy}
              className="w-48 shrink-0 bg-surface-2 border border-accent/40 rounded
                         px-1.5 py-0.5 text-xs text-neutral-400 placeholder-neutral-600 font-mono
                         focus:outline-none focus:border-accent disabled:opacity-40"
            />
          )}
        </>
      )}

      <button
        onClick={onCommit}
        disabled={busy}
        title="Confirm (Enter)"
        className="p-0.5 text-accent hover:text-accent-hover focus:outline-none
                   disabled:opacity-40 shrink-0 transition-colors"
      >
        {busy
          ? <span className="text-[11px] leading-none w-3 inline-block text-center">…</span>
          : <CheckIcon className="w-3 h-3" />
        }
      </button>
      <button
        onClick={onCancel}
        disabled={busy}
        title="Cancel (Escape)"
        className="p-0.5 text-neutral-500 hover:text-neutral-300 focus:outline-none
                   disabled:opacity-40 shrink-0 transition-colors"
      >
        <XIcon className="w-3 h-3" />
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Virtualised row glue
// ---------------------------------------------------------------------------

/**
 * Shared, memoised props passed once per render to every row.  Stored on
 * react-window's `itemData` so we don't capture the closure inside `Row`
 * directly (which would force re-render on every parent re-render).
 */
interface RowData {
  items:           FolderItem[];
  selectedId:      number | null;
  renamingId:      number | null;
  renameValue:     string;
  renamePlaceholder: string;
  dragDisabled:    boolean;
  onSelect:        (item: FolderItem) => void;
  onNavigate:      (item: FolderItem) => void;
  onStartRename:   (item: FolderItem) => void;
  onRenameChange:  (v: string) => void;
  onRenameCommit:  () => void;
  onRenameCancel:  () => void;
}

/**
 * Per-row renderer fed to FixedSizeList.  Wrapped in `memo` so virtualised
 * rows that scroll back into view re-use their previous render when their
 * inputs (item identity + selection bit) haven't changed.
 *
 * The `style` prop comes from react-window; it's an absolutely-positioned
 * box that places the row at the right vertical offset inside the inner
 * track.  We forward it to the SortableItemRow wrapper.
 */
const VirtualRow = memo(function VirtualRow(props: ListChildComponentProps<RowData>) {
  const { index, style, data } = props;
  const item = data.items[index];
  if (!item) return null;

  const selected   = item.id === data.selectedId;
  const isRenaming = item.id === data.renamingId;

  return (
    <SortableItemRow
      style={style}
      item={item}
      selected={selected}
      isRenaming={isRenaming}
      renameValue={data.renameValue}
      renamePlaceholder={data.renamePlaceholder}
      dragDisabled={data.dragDisabled}
      onRenameChange={data.onRenameChange}
      onRenameCommit={data.onRenameCommit}
      onRenameCancel={data.onRenameCancel}
      onSelect={() => data.onSelect(item)}
      onNavigate={() => data.onNavigate(item)}
      onStartRename={() => data.onStartRename(item)}
    />
  );
});

// ---------------------------------------------------------------------------
// Sortable item row (wraps ItemRow with dnd-kit)
// ---------------------------------------------------------------------------

function SortableItemRow(props: ItemRowProps & { dragDisabled: boolean; style?: CSSProperties }) {
  const { dragDisabled, item, style, ...rest } = props;
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: item.id, disabled: dragDisabled });

  // Compose react-window's absolute-positioning style with dnd-kit's
  // transform.  `style` from react-window is what places the row at the
  // correct y-offset inside the virtualised track; we must keep it.
  const composedStyle: CSSProperties = {
    ...style,
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div ref={setNodeRef} style={composedStyle}>
      <ItemRow
        item={item}
        isDragging={isDragging}
        dragDisabled={dragDisabled}
        dragHandleProps={{ ...attributes, ...listeners }}
        {...rest}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Item row
// ---------------------------------------------------------------------------

interface ItemRowProps {
  item:           FolderItem;
  selected:       boolean;
  isRenaming:     boolean;
  renameValue:    string;
  renamePlaceholder: string;
  onRenameChange: (v: string) => void;
  onRenameCommit: () => void;
  onRenameCancel: () => void;
  onSelect:       () => void;
  onNavigate:     () => void;
  onStartRename:  () => void;
}

function ItemRow({
  item,
  selected,
  isRenaming,
  renameValue,
  renamePlaceholder,
  onRenameChange,
  onRenameCommit,
  onRenameCancel,
  onSelect,
  onNavigate,
  onStartRename,
  isDragging    = false,
  dragDisabled  = false,
  dragHandleProps = {},
}: ItemRowProps & {
  isDragging?:     boolean;
  dragDisabled?:   boolean;
  dragHandleProps?: Record<string, unknown>;
}) {
  // Double-click on the row navigates folders (existing behaviour); double-
  // click on the title cell starts inline rename instead.  We can't simply
  // forward both because the title cell is nested inside the row's hit
  // surface, so the title-cell handler stops propagation when it fires.
  const canRename = item.kind !== "separator";

  return (
    <div
      role="row"
      data-item-row
      aria-selected={selected}
      onClick={isRenaming ? undefined : onSelect}
      onDoubleClick={isRenaming ? undefined : onNavigate}
      className={`flex items-center gap-1 px-2 py-1 text-xs h-full
                  transition-colors select-none
                  ${isDragging   ? "opacity-40 bg-surface-4"  : ""}
                  ${isRenaming   ? "bg-surface-4"              : ""}
                  ${selected && !isRenaming && !isDragging
                    ? "bg-surface-4 text-neutral-100"
                    : !isRenaming && !isDragging
                      ? "cursor-pointer hover:bg-surface-3 text-neutral-300"
                      : ""
                  }`}
    >
      {/* Drag handle */}
      <span
        {...dragHandleProps}
        className={`w-4 shrink-0 flex items-center justify-center
                    ${dragDisabled
                      ? "opacity-0 pointer-events-none"
                      : "cursor-grab text-neutral-700 hover:text-neutral-500 active:cursor-grabbing"
                    }`}
        onClick={(e) => e.stopPropagation()}
      >
        <GripIcon className="w-3 h-3" />
      </span>

      {/* Kind icon: selection-only column; explicitly does NOT start rename. */}
      <span
        className="w-4 shrink-0 flex items-center justify-center"
        onDoubleClick={(e) => e.stopPropagation()}
      >
        {item.kind === "folder" ? (
          <FolderIcon className={`w-3.5 h-3.5 ${selected ? "text-accent" : "text-accent/70"}`} />
        ) : item.kind === "separator" ? (
          <SeparatorIcon className="w-3.5 h-3.5 text-neutral-600" />
        ) : (
          <LinkIcon className={`w-3.5 h-3.5 ${selected ? "text-neutral-300" : "text-neutral-500"}`} />
        )}
      </span>

      {/* Title: editable when renaming, double-click starts rename. */}
      {isRenaming ? (
        <input
          autoFocus
          data-rename-input
          placeholder={renamePlaceholder}
          value={renameValue}
          onChange={(e) => onRenameChange(e.target.value)}
          onBlur={onRenameCommit}
          onKeyDown={(e: ReactKE<HTMLInputElement>) => {
            if (e.key === "Enter")  { e.preventDefault(); onRenameCommit(); }
            if (e.key === "Escape") { e.preventDefault(); onRenameCancel(); }
          }}
          onClick={(e) => e.stopPropagation()}
          onFocus={(e) => e.currentTarget.select()}
          className="flex-1 min-w-0 bg-surface-2 border border-accent rounded
                     px-1 py-0 text-xs text-neutral-100 focus:outline-none"
        />
      ) : (
        <span
          data-title-cell
          className="flex-1 min-w-0 truncate"
          onDoubleClick={(e) => {
            if (!canRename) return;
            e.stopPropagation();
            onStartRename();
          }}
        >
          {item.title || <em className="text-neutral-600 not-italic">untitled</em>}
        </span>
      )}

      {/* Domain, audit P2 #21: neutral-500 (~3.7:1) → neutral-400 (~5.5:1) */}
      {!isRenaming && (
        <span className="w-36 shrink-0 truncate text-neutral-400">{item.domain ?? ""}</span>
      )}

      {/* Date added, audit P2 #21: neutral-600 (~3:1) → neutral-400 (~5.5:1) */}
      {!isRenaming && (
        <span className="w-28 shrink-0 text-neutral-400">
          {item.add_date ? formatDate(item.add_date) : ""}
        </span>
      )}
    </div>
  );
}

function formatDate(unix: number): string {
  return new Date(unix * 1000).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}
