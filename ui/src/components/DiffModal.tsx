/**
 * Compare two open tabs and present a three-bucket diff view (Tools →
 * Compare tabs…, Ctrl+Shift+D). The diff itself is computed by
 * `lantern-core::diff`; this view picks the tabs and renders the result.
 */

import { useEffect, useState, type ReactNode } from "react";

import { ipc } from "../ipc";
import type {
  BookmarkSnapshotView,
  DocDiffReport,
  ModifiedBookmarkView,
  TabId,
  TabInfo,
} from "../ipc/types";
import { Modal, ModalHeader, primaryButton } from "./Modal";

interface DiffModalProps {
  open: boolean;
  tabs: TabInfo[];
  /** Initially-selected left tab; defaults to the first open tab. */
  initialLeft?: TabId | null;
  /** Initially-selected right tab; defaults to the second open tab. */
  initialRight?: TabId | null;
  onClose: () => void;
}

export function DiffModal({
  open,
  tabs,
  initialLeft,
  initialRight,
  onClose,
}: DiffModalProps) {
  const [leftId, setLeftId] = useState<TabId | null>(null);
  const [rightId, setRightId] = useState<TabId | null>(null);
  const [report, setReport] = useState<DocDiffReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setLeftId(initialLeft ?? tabs[0]?.id ?? null);
    setRightId(initialRight ?? tabs[1]?.id ?? tabs[0]?.id ?? null);
    setReport(null);
    setError(null);
  }, [open, initialLeft, initialRight, tabs]);

  const canCompare = leftId !== null && rightId !== null && leftId !== rightId;

  const runCompare = async () => {
    if (!canCompare) return;
    setLoading(true);
    setError(null);
    setReport(null);
    try {
      setReport(await ipc.compareTabs(leftId, rightId));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  if (!open) return null;

  const total = report
    ? report.added.length + report.removed.length + report.modified.length
    : 0;

  return (
    <Modal
      label="Compare tabs"
      onClose={onClose}
      className="flex flex-col w-[820px] h-[600px] max-h-[90vh]"
    >
      <ModalHeader title="Compare tabs" closeLabel="Close compare" onClose={onClose} />

      <div className="px-4 py-3 border-b border-neutral-800 flex items-center gap-3 shrink-0">
        <TabPicker label="Left" tabs={tabs} value={leftId} onChange={setLeftId} />
        <span className="text-neutral-600 text-xs">vs</span>
        <TabPicker label="Right" tabs={tabs} value={rightId} onChange={setRightId} />
        <button
          onClick={runCompare}
          disabled={!canCompare || loading}
          className={`ml-auto px-3 py-1 ${primaryButton}`}
        >
          {loading ? "Comparing…" : "Compare"}
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-4 py-3 min-h-0">
        {error && <p className="text-xs text-danger leading-snug mb-2">{error}</p>}
        {!report && !loading && !error && (
          <p className="text-xs text-neutral-600 italic">
            {tabs.length < 2
              ? "Open at least two documents to compare."
              : "Pick two tabs above and press Compare."}
          </p>
        )}
        {report && total === 0 && (
          <p className="text-xs text-neutral-500">
            No differences; the two tabs are bookmark-equivalent.
          </p>
        )}
        {report && total > 0 && (
          <div className="space-y-4">
            {report.added.length > 0 && (
              <SnapshotBucket
                label="Added"
                detail={`Present in ${report.right_title} only`}
                tone="text-diff-added bg-diff-added/10"
                rows={report.added}
              />
            )}
            {report.removed.length > 0 && (
              <SnapshotBucket
                label="Removed"
                detail={`Present in ${report.left_title} only`}
                tone="text-diff-removed bg-diff-removed/10"
                rows={report.removed}
              />
            )}
            {report.modified.length > 0 && <ModifiedBucket rows={report.modified} />}
          </div>
        )}
      </div>
    </Modal>
  );
}

function TabPicker({
  label,
  tabs,
  value,
  onChange,
}: {
  label: string;
  tabs: TabInfo[];
  value: TabId | null;
  onChange: (next: TabId | null) => void;
}) {
  return (
    <label className="flex items-center gap-2 text-xs text-neutral-400">
      <span className="uppercase tracking-wider text-[10px] text-neutral-500">{label}</span>
      <select
        value={value ?? ""}
        onChange={(e) => onChange(e.target.value === "" ? null : Number(e.target.value))}
        className="bg-surface-3 border border-neutral-700 rounded
                   px-2 py-1 text-xs text-neutral-200
                   focus:outline-none focus:ring-1 focus:ring-accent
                   max-w-48 truncate"
      >
        <option value="">(pick a tab)</option>
        {tabs.map((t) => (
          <option key={t.id} value={t.id}>
            {t.title}
          </option>
        ))}
      </select>
    </label>
  );
}

function Bucket({
  label,
  tone,
  count,
  detail,
  children,
}: {
  label: string;
  tone: string;
  count: number;
  detail: string;
  children: ReactNode;
}) {
  return (
    <section>
      <header className="flex items-baseline gap-2 mb-1">
        <span className={`text-[10px] font-medium uppercase tracking-wider px-1.5 py-0.5 rounded ${tone}`}>
          {label}
        </span>
        <span className="text-xs text-neutral-300 font-medium">{count}</span>
        <span className="text-[10px] text-neutral-600">{detail}</span>
      </header>
      <ul className="border border-neutral-800 rounded divide-y divide-neutral-800">{children}</ul>
    </section>
  );
}

function SnapshotBucket({
  label,
  detail,
  tone,
  rows,
}: {
  label: string;
  detail: string;
  tone: string;
  rows: BookmarkSnapshotView[];
}) {
  return (
    <Bucket label={label} tone={tone} count={rows.length} detail={detail}>
      {rows.map((r) => (
        <li key={r.url} className="px-2 py-1.5">
          <p className="text-xs text-neutral-200 truncate">
            {r.title || <em className="text-neutral-600 not-italic">(untitled)</em>}
          </p>
          <p className="text-[10px] font-mono text-neutral-500 truncate">{r.url}</p>
        </li>
      ))}
    </Bucket>
  );
}

function ModifiedBucket({ rows }: { rows: ModifiedBookmarkView[] }) {
  return (
    <Bucket
      label="Modified"
      tone="text-warn bg-warn/10"
      count={rows.length}
      detail="Same URL, different title"
    >
      {rows.map((r) => (
        <li key={r.before.url} className="px-2 py-1.5 space-y-0.5">
          <p className="text-[10px] font-mono text-neutral-500 truncate">{r.before.url}</p>
          <p className="text-xs text-diff-removed line-through truncate">{r.before.title}</p>
          <p className="text-xs text-diff-added truncate">{r.after.title}</p>
        </li>
      ))}
    </Bucket>
  );
}
