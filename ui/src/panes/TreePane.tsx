/**
 * Left pane: collapsible folder tree (v0.0.8 progressive expansion).
 *
 * On mount the pane only fetches the document root's immediate folder
 * children via `ipc.getTreeRoot`.  When the user expands a folder the
 * children are pulled lazily through `ipc.getTreeChildren(tabId, folderId)`
 * and cached per-tab so the second expand is purely local.  This keeps the
 * initial mount O(top-level folder count) regardless of document depth,
 * which matters for the deepest documents (NFR-P-* slice 2 of v0.0.7's
 * perf hardening; see CHANGELOG entry).
 *
 * Clicking a folder row jumps the list pane there; the breadcrumb trail is
 * built by walking back through the lazy-cache so the displayed crumbs
 * still cover all ancestors even though the wider tree was never fetched.
 *
 * v0.0.6 a11y is preserved verbatim: the expand chevron sits in 24×24,
 * carries `aria-expanded` and `aria-controls`, surfaces a focus-visible
 * ring, and ArrowRight / ArrowLeft expand-and-descend / collapse-and-ascend
 * the tree.
 *
 * v0.0.11 QoL slice 2, inline rename: F2 on a focused folder, or
 * double-click on the folder label (NOT the chevron; that still
 * expands/collapses), swaps the row for an `<input>` pre-filled with the
 * current name.  Enter commits via `ipc.renameNode`, Escape cancels, blur
 * commits.
 */

import {
  useCallback,
  useEffect,
  useState,
  type KeyboardEvent as ReactKE,
} from "react";
import { useDocuments, type FolderCrumb } from "../state/documents";
import { ipc } from "../ipc";
import { FolderIcon, ChevronRightIcon } from "../components/Icons";
import { SkeletonRow } from "../components/Skeleton";
import { EmptyState, EmptyFolderIcon } from "../components/EmptyState";
import { useT } from "../i18n/I18nProvider";
import { useToast } from "../hooks/useToast";
import type { TabId, TreeNodeLazy } from "../ipc/types";

// ---------------------------------------------------------------------------
// Local view-model: a TreeNodeLazy plus per-row expand/loaded flags.
// ---------------------------------------------------------------------------

interface TreeNodeLazyView extends TreeNodeLazy {
  /** True while the row is in expanded state.  Independent of `loaded`. */
  expanded: boolean;
  /** True after children have been fetched at least once. */
  loaded: boolean;
}

function decorate(rows: TreeNodeLazy[]): TreeNodeLazyView[] {
  return rows.map((r) => ({ ...r, expanded: false, loaded: false }));
}

// ---------------------------------------------------------------------------

export default function TreePane() {
  const { activeTab, folderStack, treeVersion, jumpToFolder, refreshTree } =
    useDocuments();
  const activeFolderId = folderStack.at(-1)?.id ?? 0;
  const t = useT();
  const { toast } = useToast();

  const [topLevel, setTopLevel] = useState<TreeNodeLazyView[]>([]);
  const [childrenCache, setChildrenCache] = useState<
    Record<number, TreeNodeLazyView[]>
  >({});
  const [loading, setLoading] = useState(true);

  // ── Inline rename state.  `renameNodeId` is the id of the folder
  //    currently being renamed; `renameValue` is the input buffer. ─────
  const [renameNodeId, setRenameNodeId] = useState<number | null>(null);
  const [renameValue, setRenameValue] = useState("");

  // ── Mount / active-tab change / structural-edit refresh: reload the
  //    root.  The cache is per-tab, so we wipe it on every reload (a
  //    delete may have removed a folder we have cached children for). ─
  useEffect(() => {
    let cancelled = false;
    if (activeTab === null) {
      setTopLevel([]);
      setChildrenCache({});
      setLoading(false);
      return;
    }
    setLoading(true);
    setChildrenCache({});
    void ipc.getTreeRoot(activeTab).then((rows) => {
      if (!cancelled) {
        setTopLevel(decorate(rows));
        setLoading(false);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [activeTab, treeVersion]);

  // ── Lazy fetch helper.  Returns the fresh children rows. ────────────
  const ensureLoaded = useCallback(
    async (parentId: number): Promise<TreeNodeLazyView[]> => {
      if (childrenCache[parentId]) return childrenCache[parentId];
      if (activeTab === null) return [];
      const rows = await ipc.getTreeChildren(activeTab, parentId);
      const decorated = decorate(rows);
      setChildrenCache((prev) => ({ ...prev, [parentId]: decorated }));
      return decorated;
    },
    [activeTab, childrenCache],
  );

  // ── Toggle one row's expand state, fetching children on first open. ─
  const toggleExpand = useCallback(
    async (id: number, depth: number) => {
      // Top-level toggle.
      if (depth === 1) {
        const row = topLevel.find((r) => r.id === id);
        if (!row) return;
        if (!row.expanded && !row.loaded && row.has_children) {
          await ensureLoaded(id);
        }
        setTopLevel((rows) =>
          rows.map((r) =>
            r.id === id
              ? { ...r, expanded: !r.expanded, loaded: r.loaded || r.has_children }
              : r,
          ),
        );
        return;
      }

      // Nested toggle: search every cached bucket for the row.
      let bucket: number | null = null;
      for (const [parentKey, rows] of Object.entries(childrenCache)) {
        if (rows.some((r) => r.id === id)) {
          bucket = Number(parentKey);
          break;
        }
      }
      if (bucket === null) return;
      const row = childrenCache[bucket].find((r) => r.id === id);
      if (!row) return;
      if (!row.expanded && !row.loaded && row.has_children) {
        await ensureLoaded(id);
      }
      setChildrenCache((prev) => ({
        ...prev,
        [bucket as number]: prev[bucket as number].map((r) =>
          r.id === id
            ? { ...r, expanded: !r.expanded, loaded: r.loaded || r.has_children }
            : r,
        ),
      }));
    },
    [topLevel, childrenCache, ensureLoaded],
  );

  // ── Build the FolderCrumb path for a folder id by walking the cache.
  //    Falls back to a single-crumb path with the folder's own row if we
  //    can't find a parent (defensive: shouldn't happen in practice
  //    because the user must expand into the row to see it). ──────────
  const buildCrumbs = useCallback(
    (targetId: number, name: string): FolderCrumb[] => {
      // Walk parent links by scanning the cache for an entry that lists
      // `targetId` as a child.
      const findParent = (childId: number): number | null => {
        for (const r of topLevel) if (r.id === childId) return null; // top-level → no parent
        for (const [parentKey, rows] of Object.entries(childrenCache)) {
          if (rows.some((r) => r.id === childId)) return Number(parentKey);
        }
        return null;
      };
      const findName = (id: number): string | null => {
        const top = topLevel.find((r) => r.id === id);
        if (top) return top.name;
        for (const rows of Object.values(childrenCache)) {
          const hit = rows.find((r) => r.id === id);
          if (hit) return hit.name;
        }
        return null;
      };

      const trail: FolderCrumb[] = [{ id: targetId, name: name || "Folder" }];
      let cur = targetId;
      // Bound the loop to avoid pathological infinite chains.
      for (let i = 0; i < 256; i += 1) {
        const parent = findParent(cur);
        if (parent === null) break;
        const parentName = findName(parent) ?? "Folder";
        trail.unshift({ id: parent, name: parentName });
        cur = parent;
      }
      return trail;
    },
    [topLevel, childrenCache],
  );

  const handleSelect = useCallback(
    (targetId: number, name: string) => {
      if (targetId === 0) {
        jumpToFolder([]);
        return;
      }
      jumpToFolder(buildCrumbs(targetId, name));
    },
    [jumpToFolder, buildCrumbs],
  );

  // ── Rename helpers ──────────────────────────────────────────────────
  const startRename = useCallback((id: number, currentName: string) => {
    setRenameNodeId(id);
    setRenameValue(currentName);
  }, []);

  const cancelRename = useCallback(() => {
    setRenameNodeId(null);
  }, []);

  const commitRename = useCallback(async () => {
    if (renameNodeId === null || activeTab === null) {
      setRenameNodeId(null);
      return;
    }
    const trimmed = renameValue.trim();
    const id = renameNodeId;
    // Optimistically clear before the await so a failure path doesn't
    // strand the input.
    setRenameNodeId(null);
    if (!trimmed) return;
    try {
      await ipc.renameNode(activeTab, id, trimmed);
      await refreshTree();
    } catch (e) {
      toast(
        e instanceof Error ? e.message : "Could not rename folder",
        "error",
      );
    }
  }, [renameNodeId, renameValue, activeTab, refreshTree, toast]);

  if (activeTab === null || loading) {
    // While the initial getTreeRoot call is in flight we render a stack of
    // skeleton rows so the pane's layout stays stable instead of collapsing
    // to a single "Loading…" line.  v0.0.11 QoL slice 1.
    return (
      <div
        className="flex-1 overflow-y-auto py-1"
        aria-busy="true"
        aria-label={t("loading.generic")}
      >
        {Array.from({ length: 6 }).map((_, i) => (
          <SkeletonRow key={i} />
        ))}
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto py-1">
      {/* Root entry */}
      <button
        onClick={() => handleSelect(0, "Bookmarks")}
        className={`w-full flex items-center gap-1.5 px-2 py-1 text-xs text-left
                    transition-colors focus:outline-none focus-visible:ring-1
                    focus-visible:ring-accent
                    ${activeFolderId === 0
                      ? "bg-surface-4 text-neutral-100"
                      : "text-neutral-400 hover:bg-surface-3 hover:text-neutral-200"
                    }`}
      >
        <FolderIcon className="w-3.5 h-3.5 shrink-0 text-accent" />
        <span className="truncate">Bookmarks</span>
      </button>

      {topLevel.length === 0 ? (
        <EmptyState
          icon={<EmptyFolderIcon />}
          title={t("empty.tree")}
          ariaLabel={t("empty.tree")}
          className="!py-8"
        />
      ) : (
        topLevel.map((node) => (
          <TreeNodeRow
            key={node.id}
            activeTab={activeTab}
            node={node}
            depth={1}
            activeFolderId={activeFolderId}
            childrenCache={childrenCache}
            renameNodeId={renameNodeId}
            renameValue={renameValue}
            renamePlaceholder={t("rename.placeholder")}
            onToggle={toggleExpand}
            onSelect={handleSelect}
            onStartRename={startRename}
            onRenameChange={setRenameValue}
            onRenameCommit={commitRename}
            onRenameCancel={cancelRename}
          />
        ))
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------

interface TreeNodeRowProps {
  activeTab: TabId;
  node: TreeNodeLazyView;
  depth: number;
  activeFolderId: number;
  childrenCache: Record<number, TreeNodeLazyView[]>;
  renameNodeId: number | null;
  renameValue: string;
  renamePlaceholder: string;
  onToggle: (id: number, depth: number) => Promise<void>;
  onSelect: (id: number, name: string) => void;
  onStartRename: (id: number, currentName: string) => void;
  onRenameChange: (value: string) => void;
  onRenameCommit: () => void;
  onRenameCancel: () => void;
}

function TreeNodeRow({
  activeTab: _activeTab,
  node,
  depth,
  activeFolderId,
  childrenCache,
  renameNodeId,
  renameValue,
  renamePlaceholder,
  onToggle,
  onSelect,
  onStartRename,
  onRenameChange,
  onRenameCommit,
  onRenameCancel,
}: TreeNodeRowProps) {
  const active = node.id === activeFolderId;
  const hasChildren = node.has_children;
  const isRenaming = node.id === renameNodeId;

  // Stable id for the children container so the toggle button can advertise
  // the relationship via `aria-controls` (NFR-A-2).
  const childrenId = `tree-children-${node.id}`;
  const cachedChildren = childrenCache[node.id];

  const handleLabelKeyDown = (e: ReactKE<HTMLButtonElement>) => {
    if (e.key === "F2") {
      e.preventDefault();
      onStartRename(node.id, node.name);
    }
  };

  return (
    <div>
      <div
        className={`flex items-center text-xs transition-colors
                    ${active
                      ? "bg-surface-4 text-neutral-100"
                      : "text-neutral-400 hover:bg-surface-3 hover:text-neutral-200"
                    }`}
        style={{ paddingLeft: `${8 + (depth - 1) * 14}px` }}
      >
        {/* Expand/collapse toggle.  Hit area is at least 24×24 (NFR-A-5) and
            the button advertises its expanded state and target container id
            for screen readers. */}
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            void onToggle(node.id, depth);
          }}
          aria-expanded={hasChildren ? node.expanded : undefined}
          aria-controls={hasChildren ? childrenId : undefined}
          aria-label={node.expanded ? "Collapse" : "Expand"}
          tabIndex={hasChildren ? 0 : -1}
          className={`inline-flex items-center justify-center shrink-0
                      min-w-[24px] min-h-[24px] mr-0.5 rounded
                      transition-transform focus:outline-none
                      focus-visible:ring-1 focus-visible:ring-accent
                      ${hasChildren ? "opacity-60 hover:opacity-100" : "opacity-0 pointer-events-none"}`}
        >
          <ChevronRightIcon
            className={`w-2.5 h-2.5 transition-transform ${node.expanded ? "rotate-90" : ""}`}
          />
        </button>

        {/* Folder button: single-click navigates, double-click on the
            label triggers inline rename, F2 also starts rename.  In
            rename mode we render an input in place of the label. */}
        {isRenaming ? (
          <input
            autoFocus
            data-rename-input
            placeholder={renamePlaceholder}
            value={renameValue}
            onChange={(e) => onRenameChange(e.target.value)}
            onBlur={onRenameCommit}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                onRenameCommit();
              } else if (e.key === "Escape") {
                e.preventDefault();
                onRenameCancel();
              }
            }}
            onFocus={(e) => e.currentTarget.select()}
            onClick={(e) => e.stopPropagation()}
            className="flex-1 min-w-0 mr-2 bg-surface-2 border border-accent rounded
                       px-1 py-0 text-xs text-neutral-100 focus:outline-none"
          />
        ) : (
          <button
            type="button"
            onClick={() => onSelect(node.id, node.name)}
            onDoubleClick={(e) => {
              e.stopPropagation();
              onStartRename(node.id, node.name);
            }}
            onKeyDown={handleLabelKeyDown}
            className="flex-1 flex items-center gap-1.5 py-0.5 pr-2 text-left
                       focus:outline-none focus-visible:ring-1
                       focus-visible:ring-accent rounded truncate"
          >
            <FolderIcon className={`w-3.5 h-3.5 shrink-0 ${active ? "text-accent" : "text-accent/60"}`} />
            <span className="truncate">{node.name || "(unnamed)"}</span>
          </button>
        )}
      </div>

      {hasChildren && (
        <div id={childrenId} hidden={!node.expanded}>
          {node.expanded && cachedChildren?.map((child) => (
            <TreeNodeRow
              key={child.id}
              activeTab={_activeTab}
              node={child}
              depth={depth + 1}
              activeFolderId={activeFolderId}
              childrenCache={childrenCache}
              renameNodeId={renameNodeId}
              renameValue={renameValue}
              renamePlaceholder={renamePlaceholder}
              onToggle={onToggle}
              onSelect={onSelect}
              onStartRename={onStartRename}
              onRenameChange={onRenameChange}
              onRenameCommit={onRenameCommit}
              onRenameCancel={onRenameCancel}
            />
          ))}
        </div>
      )}
    </div>
  );
}
