/**
 * Inline change-set preview overlay.
 *
 * Shows the proposed changes from `run_pass`, lets the user approve/reject
 * each one individually (or in bulk via All/None), and calls
 * `apply_changeset` when the user hits Apply.
 */

import { useState } from "react";
import { ipc } from "../ipc";
import { useDocuments } from "../state/documents";
import { useFocusTrap } from "../hooks/useFocusTrap";
import { useToast } from "../hooks/useToast";
import type { ChangeSetPreview } from "../ipc/types";
import { ChangeRow } from "./ChangeRow";
import { SkeletonText } from "./Skeleton";

export interface PreviewPanelProps {
  preview: ChangeSetPreview;
  tabId: number;
  onDone: (message: string) => void;
  onCancel: () => void;
}

export function PreviewPanel({ preview, tabId, onDone, onCancel }: PreviewPanelProps) {
  const { refreshTabs, refreshTree, refreshList } = useDocuments();
  const { toast } = useToast();
  const [approvals, setApprovals] = useState<boolean[]>(
    preview.changes.map((c) => c.approved),
  );
  const [applying, setApplying] = useState(false);

  // Trap focus inside the overlay while it's open and restore focus to
  // whatever opened it (e.g. the "Run pass" button) when it closes.  The
  // PreviewPanel is only rendered while the overlay is open, so we pass
  // `true` for `active` and let the parent unmount us to deactivate.
  const trapRef = useFocusTrap<HTMLDivElement>(true, onCancel);

  const toggle = (i: number) =>
    setApprovals((prev) => prev.map((a, j) => (j === i ? !a : a)));

  const approvedCount = approvals.filter(Boolean).length;

  const handleApply = async () => {
    setApplying(true);
    try {
      const report = await ipc.applyChangeset(tabId, preview.changeset_id, approvals);
      await Promise.all([refreshTabs(), refreshTree(), refreshList()]);
      onDone(
        `Applied ${report.applied_count} change${report.applied_count !== 1 ? "s" : ""}` +
          (report.skipped_count > 0 ? `, skipped ${report.skipped_count}.` : "."),
      );
    } catch (e) {
      // v0.0.11 QoL slice 1: surface apply failures via the global toast
      // surface instead of swallowing them silently.  We keep the overlay
      // open so the user can adjust their approvals and try again.
      toast(String(e), "error");
      setApplying(false);
    }
  };

  return (
    <div
      ref={trapRef}
      role="dialog"
      aria-modal="true"
      aria-label="Proposed changes"
      className="absolute inset-0 bg-surface-0/90 backdrop-blur-sm z-10 flex flex-col"
    >
      <div className="flex items-center justify-between px-3 py-2 border-b border-neutral-800 shrink-0">
        <div>
          <span className="text-xs font-semibold text-neutral-200">
            {preview.changes.length} proposed change
            {preview.changes.length !== 1 ? "s" : ""}
          </span>
          <span className="ml-2 text-[10px] text-neutral-500">
            {preview.rule_set_name}
          </span>
        </div>
        <button
          onClick={onCancel}
          className="text-xs text-neutral-500 hover:text-neutral-300 focus:outline-none"
        >
          Discard
        </button>
      </div>

      <div className="flex items-center gap-3 px-3 py-1.5 border-b border-neutral-800/50 shrink-0">
        <button
          onClick={() => setApprovals(approvals.map(() => true))}
          aria-label="Select all proposed changes"
          className="text-[10px] text-neutral-500 hover:text-neutral-300"
        >
          All
        </button>
        <button
          onClick={() => setApprovals(approvals.map(() => false))}
          aria-label="Deselect all proposed changes"
          className="text-[10px] text-neutral-500 hover:text-neutral-300"
        >
          None
        </button>
        <span className="ml-auto text-[10px] text-neutral-600">
          {approvedCount} selected
        </span>
      </div>

      <div className="flex-1 overflow-y-auto divide-y divide-neutral-800/50">
        {applying ? (
          // While the apply IPC is running, render a skeleton block so the
          // overlay doesn't visually freeze on the existing rows.  v0.0.11
          // QoL slice 1.
          <div className="p-3" aria-busy="true">
            <SkeletonText lines={4} />
          </div>
        ) : (
          preview.changes.map((change, i) => (
            <ChangeRow
              key={i}
              change={change}
              approved={approvals[i]}
              onToggle={() => toggle(i)}
            />
          ))
        )}
      </div>

      <div className="p-3 border-t border-neutral-800 shrink-0">
        <button
          onClick={handleApply}
          disabled={applying || approvedCount === 0}
          className="w-full py-1.5 rounded bg-accent hover:bg-accent-hover
                     disabled:opacity-40 text-on-accent font-medium text-xs
                     transition-colors focus:outline-none focus-visible:ring-2
                     focus-visible:ring-accent"
        >
          {applying
            ? "Applying…"
            : `Apply ${approvedCount} change${approvedCount !== 1 ? "s" : ""}`}
        </button>
      </div>
    </div>
  );
}
