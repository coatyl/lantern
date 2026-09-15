/**
 * Cross-document merge picker: pick subtrees from any open tabs, choose a
 * conflict strategy, and produce a brand-new merged document in a fresh tab.
 *
 * Surfaced via Tools → Merge documents…  Implements the user-facing surface
 * of ADR-0009.  All of the heavy lifting happens in
 * `lantern_core::model::merge::build_merged_document`; this view just lets
 * the user assemble a `MergePlan` and routes the resulting tab id to the
 * caller via `onMerged`.
 */

import { useEffect, useState } from "react";

import { ipc } from "../ipc";
import type {
  ConflictStrategyView,
  MergePickRequest,
  NodeId,
  TabId,
  TabInfo,
  TreeNode,
  TreeView,
} from "../ipc/types";
import { useFocusTrap } from "../hooks/useFocusTrap";
import { useToast } from "../hooks/useToast";
import { XIcon } from "./Icons";

interface MergePickerModalProps {
  open: boolean;
  tabs: TabInfo[];
  onClose: () => void;
  /** Called with the new tab ID after a successful merge. */
  onMerged?: (newTabId: TabId) => void;
}

interface PickEntry extends MergePickRequest {
  /** Display label captured at pick time so the chip survives tab close. */
  label: string;
}

const DEFAULT_ROOT_NAME = "Merged bookmarks";

const STRATEGY_LABEL: Record<ConflictStrategyView, string> = {
  keep_first: "Keep first",
  keep_newest: "Keep newest",
  keep_both: "Keep both",
};

const STRATEGY_TIP: Record<ConflictStrategyView, string> = {
  keep_first: "Drop a bookmark if a sibling with the same URL was already added.",
  keep_newest: "Keep the bookmark with the most-recent add date.",
  keep_both: "Keep both: duplicates get a “ (2)” suffix to disambiguate.",
};

export function MergePickerModal({
  open,
  tabs,
  onClose,
  onMerged,
}: MergePickerModalProps) {
  const { toast } = useToast();
  const [picks, setPicks] = useState<PickEntry[]>([]);
  const [strategy, setStrategy] = useState<ConflictStrategyView>("keep_first");
  const [rootName, setRootName] = useState<string>(DEFAULT_ROOT_NAME);
  const [merging, setMerging] = useState(false);

  // Per-tab tree cache.  Loaded lazily when the user expands a tab section.
  const [trees, setTrees] = useState<Record<TabId, TreeView | null>>({});
  const [loadingTabs, setLoadingTabs] = useState<Record<TabId, boolean>>({});

  // Reset state when the modal is reopened.
  useEffect(() => {
    if (!open) return;
    setPicks([]);
    setStrategy("keep_first");
    setRootName(DEFAULT_ROOT_NAME);
    setMerging(false);
  }, [open]);

  const loadTreeFor = async (tabId: TabId) => {
    if (trees[tabId] !== undefined && trees[tabId] !== null) return;
    setLoadingTabs((prev) => ({ ...prev, [tabId]: true }));
    try {
      const tree = await ipc.getTree(tabId);
      setTrees((prev) => ({ ...prev, [tabId]: tree }));
    } catch (e) {
      // Route load failures through the toast surface so the picker stays
      // usable for any other tabs the user wants to expand.  v0.0.11 QoL
      // slice 1.
      toast(`Could not load tree for tab ${tabId}: ${e}`, "error");
    } finally {
      setLoadingTabs((prev) => ({ ...prev, [tabId]: false }));
    }
  };

  const addPick = (tabId: TabId, nodeId: NodeId, label: string) => {
    setPicks((prev) => {
      // Dedup exact picks (same tab + node).
      if (prev.some((p) => p.source_tab_id === tabId && p.root_node_id === nodeId)) {
        return prev;
      }
      return [...prev, { source_tab_id: tabId, root_node_id: nodeId, label }];
    });
  };

  const removePick = (idx: number) => {
    setPicks((prev) => prev.filter((_, i) => i !== idx));
  };

  const canMerge =
    picks.length > 0 && rootName.trim().length > 0 && !merging;

  const runMerge = async () => {
    if (!canMerge) return;
    setMerging(true);
    try {
      const newTab = await ipc.mergeDocuments(
        picks.map(({ source_tab_id, root_node_id }) => ({
          source_tab_id,
          root_node_id,
        })),
        strategy,
        rootName.trim(),
      );
      onMerged?.(newTab);
      onClose();
    } catch (e) {
      // Merge failures route through the toast surface; the modal stays
      // open so the user can adjust their plan and try again.  v0.0.11 QoL
      // slice 1.
      toast(String(e), "error");
    } finally {
      setMerging(false);
    }
  };

  const dialogRef = useFocusTrap<HTMLDivElement>(open, onClose);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center
                 bg-surface-0/80 backdrop-blur-sm animate-fade-in"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        className="bg-surface-1 border border-neutral-800 rounded-lg
                   shadow-2xl flex flex-col overflow-hidden
                   w-[820px] max-w-[95vw] h-[640px] max-h-[90vh]"
        role="dialog"
        aria-modal="true"
        aria-label="Merge documents"
      >
        {/* Header */}
        <div className="px-4 py-3 border-b border-neutral-800 flex items-center
                        justify-between gap-2 shrink-0">
          <h2 className="text-sm font-semibold text-neutral-100">Merge documents</h2>
          <button
            onClick={onClose}
            className="text-neutral-600 hover:text-neutral-300 rounded
                       focus:outline-none focus-visible:ring-1
                       focus-visible:ring-accent"
            aria-label="Close merge picker"
          >
            <XIcon className="w-4 h-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 min-h-0 grid grid-cols-2 gap-0">
          {/* Left: open-tab tree picker */}
          <div className="border-r border-neutral-800 overflow-y-auto px-3 py-3">
            <p className="text-[10px] uppercase tracking-wider text-neutral-500 mb-2">
              Open tabs
            </p>
            {tabs.length === 0 ? (
              <p className="text-xs text-neutral-600 italic">
                Open at least one document to merge.
              </p>
            ) : (
              <ul className="space-y-3">
                {tabs.map((t) => (
                  <TabSection
                    key={t.id}
                    tab={t}
                    tree={trees[t.id] ?? null}
                    loading={Boolean(loadingTabs[t.id])}
                    onExpand={() => loadTreeFor(t.id)}
                    onPick={(nodeId, label) => addPick(t.id, nodeId, label)}
                  />
                ))}
              </ul>
            )}
          </div>

          {/* Right: plan + controls */}
          <div className="flex flex-col px-3 py-3 min-h-0">
            <p className="text-[10px] uppercase tracking-wider text-neutral-500 mb-2">
              Picks ({picks.length})
            </p>
            <div className="flex-1 overflow-y-auto min-h-0 mb-3">
              {picks.length === 0 ? (
                <p className="text-xs text-neutral-600 italic">
                  Pick a folder on the left to add it to the merge plan.
                </p>
              ) : (
                <ul className="border border-neutral-800 rounded divide-y divide-neutral-800">
                  {picks.map((p, idx) => (
                    <li
                      key={`${p.source_tab_id}-${p.root_node_id}`}
                      className="flex items-center justify-between gap-2 px-2 py-1.5"
                    >
                      <span className="text-xs text-neutral-200 truncate">
                        {p.label}
                      </span>
                      <button
                        type="button"
                        onClick={() => removePick(idx)}
                        className="text-[10px] text-neutral-500 hover:text-neutral-200
                                   focus:outline-none focus-visible:ring-1
                                   focus-visible:ring-accent rounded px-1.5 py-0.5"
                        aria-label={`Remove pick ${p.label}`}
                      >
                        Remove
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </div>

            {/* Strategy radios */}
            <fieldset className="mb-3">
              <legend className="text-[10px] uppercase tracking-wider text-neutral-500 mb-1.5">
                Conflict strategy
              </legend>
              <div className="flex flex-col gap-1">
                {(["keep_first", "keep_newest", "keep_both"] as ConflictStrategyView[]).map((s) => (
                  <label
                    key={s}
                    className="flex items-center gap-2 text-xs text-neutral-300
                               cursor-pointer"
                    title={STRATEGY_TIP[s]}
                  >
                    <input
                      type="radio"
                      name="conflict-strategy"
                      value={s}
                      checked={strategy === s}
                      onChange={() => setStrategy(s)}
                      className="accent-accent"
                    />
                    <span>{STRATEGY_LABEL[s]}</span>
                    <span className="text-[10px] text-neutral-600 truncate">
                      {STRATEGY_TIP[s]}
                    </span>
                  </label>
                ))}
              </div>
            </fieldset>

            {/* Root name input */}
            <label className="flex flex-col gap-1 mb-3">
              <span className="text-[10px] uppercase tracking-wider text-neutral-500">
                Merged root name
              </span>
              <input
                type="text"
                value={rootName}
                onChange={(e) => setRootName(e.target.value)}
                className="bg-surface-3 border border-neutral-700 rounded
                           px-2 py-1 text-xs text-neutral-200
                           focus:outline-none focus:ring-1 focus:ring-accent"
                placeholder={DEFAULT_ROOT_NAME}
              />
            </label>

            <div className="flex items-center justify-end gap-2 mt-auto">
              <button
                type="button"
                onClick={onClose}
                className="px-3 py-1 rounded text-xs font-medium
                           border border-neutral-700 text-neutral-300
                           hover:border-neutral-500 hover:text-neutral-100
                           focus:outline-none focus-visible:ring-2
                           focus-visible:ring-accent"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={runMerge}
                disabled={!canMerge}
                className="px-3 py-1 rounded text-xs font-medium
                           bg-accent hover:bg-accent-hover text-neutral-950
                           disabled:opacity-50 transition-colors
                           focus:outline-none focus-visible:ring-2
                           focus-visible:ring-accent"
              >
                {merging ? "Merging…" : "Merge"}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Tab section: collapsible per-tab tree of folders.
// ---------------------------------------------------------------------------

interface TabSectionProps {
  tab: TabInfo;
  tree: TreeView | null;
  loading: boolean;
  onExpand: () => void;
  onPick: (nodeId: NodeId, label: string) => void;
}

function TabSection({ tab, tree, loading, onExpand, onPick }: TabSectionProps) {
  const [expanded, setExpanded] = useState(false);

  const toggle = () => {
    const willOpen = !expanded;
    setExpanded(willOpen);
    if (willOpen && !tree) onExpand();
  };

  return (
    <li className="text-xs">
      <button
        type="button"
        onClick={toggle}
        aria-expanded={expanded}
        className="w-full text-left flex items-center justify-between
                   px-2 py-1 rounded hover:bg-surface-2
                   focus:outline-none focus-visible:ring-1
                   focus-visible:ring-accent"
      >
        <span className="text-neutral-200 truncate">{tab.title}</span>
        <span className="text-[10px] text-neutral-600">
          {expanded ? "Collapse" : "Expand"}
        </span>
      </button>
      {expanded && (
        <div className="mt-1 ml-2 border-l border-neutral-800 pl-2">
          {loading && !tree ? (
            <p className="text-[10px] text-neutral-600 italic py-1">Loading…</p>
          ) : tree ? (
            <FolderTree
              node={tree.root}
              tabTitle={tab.title}
              ancestry={[]}
              onPick={onPick}
            />
          ) : (
            <p className="text-[10px] text-neutral-600 italic py-1">
              No tree available.
            </p>
          )}
        </div>
      )}
    </li>
  );
}

interface FolderTreeProps {
  node: TreeNode;
  tabTitle: string;
  ancestry: string[];
  onPick: (nodeId: NodeId, label: string) => void;
}

function FolderTree({ node, tabTitle, ancestry, onPick }: FolderTreeProps) {
  // The IPC `TreeView.root` is the document root (id 0 in core; aliased in
  // ts-rs).  We don't let the user "pick the root" because that's just the
  // whole document; they should open a new tab for that.
  const isDocumentRoot = ancestry.length === 0;
  const childAncestry = isDocumentRoot ? [tabTitle] : [...ancestry, node.name];
  const breadcrumb = isDocumentRoot ? tabTitle : `${tabTitle} › ${[...ancestry, node.name].join(" › ")}`;

  return (
    <ul className="space-y-0.5">
      {!isDocumentRoot && (
        <li className="flex items-center gap-2 py-0.5">
          <span className="text-neutral-300 truncate">{node.name}</span>
          <button
            type="button"
            onClick={() => onPick(node.id, breadcrumb)}
            className="ml-auto px-1.5 py-0.5 rounded text-[10px]
                       border border-neutral-700 text-neutral-400
                       hover:border-accent hover:text-accent
                       focus:outline-none focus-visible:ring-1
                       focus-visible:ring-accent"
          >
            Pick
          </button>
        </li>
      )}
      {node.children.length > 0 && (
        <li className={isDocumentRoot ? "" : "ml-3"}>
          <ul className="space-y-0.5">
            {node.children.map((child) => (
              <FolderTree
                key={child.id}
                node={child}
                tabTitle={tabTitle}
                ancestry={childAncestry}
                onPick={onPick}
              />
            ))}
          </ul>
        </li>
      )}
    </ul>
  );
}
