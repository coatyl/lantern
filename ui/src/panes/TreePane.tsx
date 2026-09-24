/**
 * Left pane: the folder tree, as a WAI-ARIA tree.
 *
 * Folders load lazily: the root's children on mount, a folder's children the
 * first time it is expanded (then cached for the tab), so mounting stays
 * cheap however deep the document is.
 *
 * Keyboard (one tab stop for the whole tree; focus roves between rows):
 *   ↑ / ↓        previous / next visible folder
 *   → / ←        expand, or step into the first child / collapse, or step to the parent
 *   Home / End   first / last visible folder
 *   Enter/Space  show the folder in the list
 *   F2           rename (also: double-click the name)
 */

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { useDocuments, type FolderCrumb } from "../state/documents";
import { ipc } from "../ipc";
import { FolderIcon, ChevronRightIcon } from "../components/Icons";
import { SkeletonRow } from "../components/Skeleton";
import { EmptyState, EmptyFolderIcon } from "../components/EmptyState";
import { useT } from "../i18n/I18nProvider";
import { useToast } from "../hooks/useToast";
import type { TreeNodeLazy } from "../ipc/types";

/** Id the backend uses for the document root in tree/list commands. */
const ROOT = 0;

interface VisibleRow {
  node: TreeNodeLazy;
  level: number;
  parentId: number;
}

/** Depth-first list of the rows currently on screen (the root excluded). */
function flatten(
  children: Record<number, TreeNodeLazy[]>,
  expanded: Set<number>,
  parentId = ROOT,
  level = 2,
  out: VisibleRow[] = [],
): VisibleRow[] {
  for (const node of children[parentId] ?? []) {
    out.push({ node, level, parentId });
    if (expanded.has(node.id)) flatten(children, expanded, node.id, level + 1, out);
  }
  return out;
}

export default function TreePane() {
  const { activeTab, folderStack, treeVersion, jumpToFolder, refreshTree } = useDocuments();
  const activeFolderId = folderStack.at(-1)?.id ?? ROOT;
  const t = useT();
  const { toast } = useToast();

  const [children, setChildren] = useState<Record<number, TreeNodeLazy[]>>({});
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const [loading, setLoading] = useState(true);
  const [focusedId, setFocusedId] = useState<number>(ROOT);
  const [renameId, setRenameId] = useState<number | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const rowRefs = useRef(new Map<number, HTMLDivElement>());

  // Reload on tab change and after structural edits (rename / delete /
  // apply bump `treeVersion`): a cached subtree may no longer exist.
  useEffect(() => {
    let cancelled = false;
    setChildren({});
    setExpanded(new Set());
    if (activeTab === null) {
      setLoading(false);
      return;
    }
    setLoading(true);
    void ipc.getTreeRoot(activeTab).then((rows) => {
      if (cancelled) return;
      setChildren({ [ROOT]: rows });
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [activeTab, treeVersion]);

  const rows = useMemo(() => flatten(children, expanded), [children, expanded]);
  const parentOf = useMemo(() => {
    const map = new Map<number, number>();
    for (const [parent, kids] of Object.entries(children)) {
      for (const kid of kids) map.set(kid.id, Number(parent));
    }
    return map;
  }, [children]);
  const nameOf = useCallback(
    (id: number) => {
      for (const kids of Object.values(children)) {
        const hit = kids.find((k) => k.id === id);
        if (hit) return hit.name;
      }
      return "";
    },
    [children],
  );

  const setOpen = useCallback(
    async (node: TreeNodeLazy, open: boolean) => {
      if (!node.has_children) return;
      if (open && !children[node.id] && activeTab !== null) {
        const kids = await ipc.getTreeChildren(activeTab, node.id);
        setChildren((prev) => ({ ...prev, [node.id]: kids }));
      }
      setExpanded((prev) => {
        const next = new Set(prev);
        if (open) next.add(node.id);
        else next.delete(node.id);
        return next;
      });
    },
    [activeTab, children],
  );

  const select = useCallback(
    (id: number) => {
      if (id === ROOT) {
        void jumpToFolder([]);
        return;
      }
      const trail: FolderCrumb[] = [];
      for (let cur: number | undefined = id; cur !== undefined && cur !== ROOT; cur = parentOf.get(cur)) {
        trail.unshift({ id: cur, name: nameOf(cur) || t("tree.unnamed") });
      }
      void jumpToFolder(trail);
    },
    [jumpToFolder, parentOf, nameOf, t],
  );

  const focusRow = (id: number) => {
    setFocusedId(id);
    rowRefs.current.get(id)?.focus();
  };

  const startRename = (id: number) => {
    setRenameId(id);
    setRenameValue(nameOf(id));
  };

  const commitRename = async () => {
    const id = renameId;
    const name = renameValue.trim();
    setRenameId(null);
    if (id === null || activeTab === null || !name || name === nameOf(id)) return;
    try {
      await ipc.renameNode(activeTab, id, name);
      await refreshTree();
    } catch (e) {
      toast(e instanceof Error ? e.message : String(e), "error");
    }
  };

  const onTreeKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (renameId !== null) return;
    // Visible order including the root row at index 0.
    const order = [ROOT, ...rows.map((r) => r.node.id)];
    const index = Math.max(0, order.indexOf(focusedId));
    const row = rows.find((r) => r.node.id === focusedId);
    const handled = () => e.preventDefault();

    switch (e.key) {
      case "ArrowDown":
        handled();
        focusRow(order[Math.min(index + 1, order.length - 1)]);
        break;
      case "ArrowUp":
        handled();
        focusRow(order[Math.max(index - 1, 0)]);
        break;
      case "Home":
        handled();
        focusRow(order[0]);
        break;
      case "End":
        handled();
        focusRow(order[order.length - 1]);
        break;
      case "ArrowRight":
        handled();
        if (!row) {
          if (rows.length > 0) focusRow(rows[0].node.id);
        } else if (row.node.has_children && !expanded.has(row.node.id)) {
          void setOpen(row.node, true);
        } else if (expanded.has(row.node.id)) {
          const first = children[row.node.id]?.[0];
          if (first) focusRow(first.id);
        }
        break;
      case "ArrowLeft":
        handled();
        if (row && expanded.has(row.node.id)) void setOpen(row.node, false);
        else if (row) focusRow(row.parentId);
        break;
      case "Enter":
      case " ":
        handled();
        select(focusedId);
        break;
      case "F2":
        if (focusedId !== ROOT) {
          handled();
          startRename(focusedId);
        }
        break;
    }
  };

  if (activeTab === null || loading) {
    return (
      <div className="flex-1 overflow-y-auto py-1" aria-busy="true" aria-label={t("loading.generic")}>
        {Array.from({ length: 6 }).map((_, i) => (
          <SkeletonRow key={i} />
        ))}
      </div>
    );
  }

  const rowClass = (active: boolean) =>
    `flex items-center h-7 pr-2 text-xs cursor-pointer select-none outline-none rounded-sm
     focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-accent transition-colors ${
       active ? "bg-surface-4 text-neutral-100" : "text-neutral-400 hover:bg-surface-3 hover:text-neutral-200"
     }`;

  return (
    <div className="flex-1 overflow-y-auto py-1">
      <div role="tree" aria-label={t("tree.label")} onKeyDown={onTreeKeyDown}>
        <div
          role="treeitem"
          aria-level={1}
          aria-selected={activeFolderId === ROOT}
          aria-expanded
          tabIndex={focusedId === ROOT ? 0 : -1}
          ref={(el) => {
            if (el) rowRefs.current.set(ROOT, el);
          }}
          onFocus={() => setFocusedId(ROOT)}
          onClick={() => select(ROOT)}
          className={`${rowClass(activeFolderId === ROOT)} pl-2 gap-1.5`}
        >
          <FolderIcon className="w-3.5 h-3.5 shrink-0 text-accent" />
          <span className="truncate font-medium">{t("list.root")}</span>
        </div>

        {rows.length === 0 ? (
          <EmptyState
            icon={<EmptyFolderIcon />}
            title={t("empty.tree")}
            description={t("empty.tree.description")}
            ariaLabel={t("empty.tree")}
            className="!py-8"
          />
        ) : (
          <div role="group">
            {rows.map(({ node, level }) => {
              const active = node.id === activeFolderId;
              const isOpen = expanded.has(node.id);
              return (
                <div
                  key={node.id}
                  role="treeitem"
                  aria-level={level}
                  aria-selected={active}
                  aria-expanded={node.has_children ? isOpen : undefined}
                  tabIndex={focusedId === node.id ? 0 : -1}
                  ref={(el) => {
                    if (el) rowRefs.current.set(node.id, el);
                    else rowRefs.current.delete(node.id);
                  }}
                  onFocus={() => setFocusedId(node.id)}
                  onClick={() => select(node.id)}
                  onDoubleClick={() => startRename(node.id)}
                  style={{ paddingLeft: `${4 + (level - 2) * 14}px` }}
                  className={rowClass(active)}
                >
                  <span
                    aria-hidden
                    onClick={(e) => {
                      e.stopPropagation();
                      void setOpen(node, !isOpen);
                    }}
                    onDoubleClick={(e) => e.stopPropagation()}
                    className={`inline-flex items-center justify-center w-6 h-6 shrink-0 rounded
                                ${node.has_children ? "opacity-60 hover:opacity-100 hover:bg-surface-4" : "invisible"}`}
                  >
                    <ChevronRightIcon className={`w-2.5 h-2.5 transition-transform ${isOpen ? "rotate-90" : ""}`} />
                  </span>
                  <FolderIcon className={`w-3.5 h-3.5 shrink-0 mr-1.5 ${active ? "text-accent" : "text-accent/60"}`} />
                  {renameId === node.id ? (
                    <input
                      autoFocus
                      data-rename-input
                      aria-label={t("rename.placeholder")}
                      placeholder={t("rename.placeholder")}
                      value={renameValue}
                      onChange={(e) => setRenameValue(e.target.value)}
                      onBlur={() => void commitRename()}
                      onKeyDown={(e) => {
                        e.stopPropagation();
                        if (e.key === "Enter") {
                          e.preventDefault();
                          void commitRename();
                        } else if (e.key === "Escape") {
                          e.preventDefault();
                          setRenameId(null);
                          rowRefs.current.get(node.id)?.focus();
                        }
                      }}
                      onFocus={(e) => e.currentTarget.select()}
                      onClick={(e) => e.stopPropagation()}
                      className="flex-1 min-w-0 bg-surface-2 border border-accent rounded px-1 text-xs
                                 text-neutral-100 focus:outline-none"
                    />
                  ) : (
                    <span className="truncate">{node.name || t("tree.unnamed")}</span>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
