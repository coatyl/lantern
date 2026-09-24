/**
 * Cross-document merge picker (Tools → Merge documents…): pick folders from
 * any open tabs, choose a conflict strategy, and build a new merged document
 * in a fresh tab. The merge itself happens in
 * `lantern_core::model::merge::build_merged_document`.
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
import { useToast } from "../hooks/useToast";
import { Modal, ModalHeader, primaryButton } from "./Modal";

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

const STRATEGIES: { id: ConflictStrategyView; label: string; tip: string }[] = [
  {
    id: "keep_first",
    label: "Keep first",
    tip: "Drop a bookmark if a sibling with the same URL was already added.",
  },
  {
    id: "keep_newest",
    label: "Keep newest",
    tip: "Keep the bookmark with the most-recent add date.",
  },
  {
    id: "keep_both",
    label: "Keep both",
    tip: "Keep both: duplicates get a “ (2)” suffix to disambiguate.",
  },
];

export function MergePickerModal({ open, tabs, onClose, onMerged }: MergePickerModalProps) {
  const { toast } = useToast();
  const [picks, setPicks] = useState<PickEntry[]>([]);
  const [strategy, setStrategy] = useState<ConflictStrategyView>("keep_first");
  const [rootName, setRootName] = useState(DEFAULT_ROOT_NAME);
  const [merging, setMerging] = useState(false);

  // Per-tab tree cache, filled lazily when the user expands a tab section.
  const [trees, setTrees] = useState<Record<TabId, TreeView>>({});
  const [loadingTabs, setLoadingTabs] = useState<Record<TabId, boolean>>({});

  useEffect(() => {
    if (!open) return;
    setPicks([]);
    setStrategy("keep_first");
    setRootName(DEFAULT_ROOT_NAME);
    setMerging(false);
  }, [open]);

  const loadTreeFor = async (tabId: TabId) => {
    if (trees[tabId]) return;
    setLoadingTabs((prev) => ({ ...prev, [tabId]: true }));
    try {
      const tree = await ipc.getTree(tabId);
      setTrees((prev) => ({ ...prev, [tabId]: tree }));
    } catch (e) {
      // Toast rather than block: the other tabs stay pickable.
      toast(`Could not load tree for tab ${tabId}: ${e}`, "error");
    } finally {
      setLoadingTabs((prev) => ({ ...prev, [tabId]: false }));
    }
  };

  const addPick = (tabId: TabId, nodeId: NodeId, label: string) => {
    setPicks((prev) =>
      prev.some((p) => p.source_tab_id === tabId && p.root_node_id === nodeId)
        ? prev
        : [...prev, { source_tab_id: tabId, root_node_id: nodeId, label }],
    );
  };

  const canMerge = picks.length > 0 && rootName.trim().length > 0 && !merging;

  const runMerge = async () => {
    if (!canMerge) return;
    setMerging(true);
    try {
      const newTab = await ipc.mergeDocuments(
        picks.map(({ source_tab_id, root_node_id }) => ({ source_tab_id, root_node_id })),
        strategy,
        rootName.trim(),
      );
      onMerged?.(newTab);
      onClose();
    } catch (e) {
      // The modal stays open so the user can adjust the plan and retry.
      toast(String(e), "error");
    } finally {
      setMerging(false);
    }
  };

  if (!open) return null;

  return (
    <Modal
      label="Merge documents"
      onClose={onClose}
      className="flex flex-col w-[820px] h-[640px] max-h-[90vh]"
    >
      <ModalHeader title="Merge documents" closeLabel="Close merge picker" onClose={onClose} />

      <div className="flex-1 min-h-0 grid grid-cols-2 gap-0">
        {/* Left: open-tab tree picker */}
        <div className="border-r border-neutral-800 overflow-y-auto px-3 py-3">
          <p className="text-[10px] uppercase tracking-wider text-neutral-500 mb-2">Open tabs</p>
          {tabs.length === 0 ? (
            <p className="text-xs text-neutral-600 italic">Open at least one document to merge.</p>
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
                    <span className="text-xs text-neutral-200 truncate">{p.label}</span>
                    <button
                      type="button"
                      onClick={() => setPicks((prev) => prev.filter((_, i) => i !== idx))}
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

          <fieldset className="mb-3">
            <legend className="text-[10px] uppercase tracking-wider text-neutral-500 mb-1.5">
              Conflict strategy
            </legend>
            <div className="flex flex-col gap-1">
              {STRATEGIES.map((s) => (
                <label
                  key={s.id}
                  className="flex items-center gap-2 text-xs text-neutral-300 cursor-pointer"
                  title={s.tip}
                >
                  <input
                    type="radio"
                    name="conflict-strategy"
                    value={s.id}
                    checked={strategy === s.id}
                    onChange={() => setStrategy(s.id)}
                    className="accent-accent"
                  />
                  <span>{s.label}</span>
                  <span className="text-[10px] text-neutral-600 truncate">{s.tip}</span>
                </label>
              ))}
            </div>
          </fieldset>

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
              className={`px-3 py-1 ${primaryButton}`}
            >
              {merging ? "Merging…" : "Merge"}
            </button>
          </div>
        </div>
      </div>
    </Modal>
  );
}

interface TabSectionProps {
  tab: TabInfo;
  tree: TreeView | null;
  loading: boolean;
  onExpand: () => void;
  onPick: (nodeId: NodeId, label: string) => void;
}

/** Collapsible per-tab list of pickable folders. */
function TabSection({ tab, tree, loading, onExpand, onPick }: TabSectionProps) {
  const [expanded, setExpanded] = useState(false);

  const toggle = () => {
    setExpanded(!expanded);
    if (!expanded && !tree) onExpand();
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
        <span className="text-[10px] text-neutral-600">{expanded ? "Collapse" : "Expand"}</span>
      </button>
      {expanded && (
        <div className="mt-1 ml-2 border-l border-neutral-800 pl-2">
          {loading && !tree ? (
            <p className="text-[10px] text-neutral-600 italic py-1">Loading…</p>
          ) : tree ? (
            // The document root itself is not pickable: that would just be
            // the whole document.
            <ul className="space-y-0.5">
              {tree.root.children.map((child) => (
                <FolderNode key={child.id} node={child} ancestry={[tab.title]} onPick={onPick} />
              ))}
            </ul>
          ) : (
            <p className="text-[10px] text-neutral-600 italic py-1">No tree available.</p>
          )}
        </div>
      )}
    </li>
  );
}

function FolderNode({
  node,
  ancestry,
  onPick,
}: {
  node: TreeNode;
  /** Tab title followed by the names of the enclosing folders. */
  ancestry: string[];
  onPick: (nodeId: NodeId, label: string) => void;
}) {
  const path = [...ancestry, node.name];
  return (
    <li>
      <div className="flex items-center gap-2 py-0.5">
        <span className="text-neutral-300 truncate">{node.name}</span>
        <button
          type="button"
          onClick={() => onPick(node.id, path.join(" › "))}
          className="ml-auto px-1.5 py-0.5 rounded text-[10px]
                     border border-neutral-700 text-neutral-400
                     hover:border-accent hover:text-accent
                     focus:outline-none focus-visible:ring-1
                     focus-visible:ring-accent"
        >
          Pick
        </button>
      </div>
      {node.children.length > 0 && (
        <ul className="ml-3 mt-0.5 space-y-0.5">
          {node.children.map((child) => (
            <FolderNode key={child.id} node={child} ancestry={path} onPick={onPick} />
          ))}
        </ul>
      )}
    </li>
  );
}
