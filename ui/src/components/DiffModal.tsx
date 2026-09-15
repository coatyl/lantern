/**
 * Compare two open tabs and present a three-bucket diff view.
 *
 * Surfaced via Tools → Compare tabs… (Ctrl+Shift+D).  The modal is intentionally
 * dumb: all of the diff logic happens in `lantern-core::diff`; this view just
 * picks the two tabs and renders the result.
 */

import { useEffect, useMemo, useState } from "react";

import { ipc } from "../ipc";
import type { DocDiffReport, TabId, TabInfo } from "../ipc/types";
import { useFocusTrap } from "../hooks/useFocusTrap";
import { XIcon } from "./Icons";

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

  // Auto-pick a sensible default when the modal opens.
  useEffect(() => {
    if (!open) return;
    setLeftId(initialLeft ?? tabs[0]?.id ?? null);
    setRightId(initialRight ?? tabs[1]?.id ?? tabs[0]?.id ?? null);
    setReport(null);
    setError(null);
  }, [open, initialLeft, initialRight, tabs]);

  const canCompare =
    leftId !== null && rightId !== null && leftId !== rightId;

  const runCompare = async () => {
    if (!canCompare || leftId === null || rightId === null) return;
    setLoading(true);
    setError(null);
    setReport(null);
    try {
      const r = await ipc.compareTabs(leftId, rightId);
      setReport(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const total = useMemo(() => {
    if (!report) return 0;
    return report.added.length + report.removed.length + report.modified.length;
  }, [report]);

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
                   w-[820px] max-w-[95vw] h-[600px] max-h-[90vh]"
        role="dialog"
        aria-modal="true"
        aria-label="Compare tabs"
      >
        {/* Header */}
        <div className="px-4 py-3 border-b border-neutral-800 flex items-center
                        justify-between gap-2 shrink-0">
          <h2 className="text-sm font-semibold text-neutral-100">Compare tabs</h2>
          {/* audit P2 #25: hover bg + 32x32 hit area on modal close button */}
          <button
            onClick={onClose}
            className="inline-flex items-center justify-center w-8 h-8 rounded
                       text-neutral-400 hover:text-neutral-100
                       hover:bg-neutral-800/60 transition-colors
                       focus:outline-none focus-visible:ring-1
                       focus-visible:ring-accent"
            aria-label="Close compare"
          >
            <XIcon className="w-4 h-4" />
          </button>
        </div>

        {/* Tab pickers */}
        <div className="px-4 py-3 border-b border-neutral-800 flex items-center
                        gap-3 shrink-0">
          <TabPicker
            label="Left"
            tabs={tabs}
            value={leftId}
            onChange={setLeftId}
          />
          <span className="text-neutral-600 text-xs">vs</span>
          <TabPicker
            label="Right"
            tabs={tabs}
            value={rightId}
            onChange={setRightId}
          />
          <button
            onClick={runCompare}
            disabled={!canCompare || loading}
            className="ml-auto px-3 py-1 rounded text-xs font-medium
                       bg-accent hover:bg-accent-hover text-neutral-950
                       disabled:opacity-50 transition-colors
                       focus:outline-none focus-visible:ring-2
                       focus-visible:ring-accent"
          >
            {loading ? "Comparing…" : "Compare"}
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto px-4 py-3 min-h-0">
          {error && (
            <p className="text-xs text-danger leading-snug mb-2">{error}</p>
          )}
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
                <DiffBucket
                  label="Added"
                  detail={`Present in ${report.right_title} only`}
                  count={report.added.length}
                  rowKind="added"
                  rows={report.added.map((b) => ({
                    url: b.url,
                    title: b.title,
                  }))}
                />
              )}
              {report.removed.length > 0 && (
                <DiffBucket
                  label="Removed"
                  detail={`Present in ${report.left_title} only`}
                  count={report.removed.length}
                  rowKind="removed"
                  rows={report.removed.map((b) => ({
                    url: b.url,
                    title: b.title,
                  }))}
                />
              )}
              {report.modified.length > 0 && (
                <ModifiedBucket
                  count={report.modified.length}
                  rows={report.modified}
                />
              )}
            </div>
          )}
        </div>
      </div>
    </div>
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
      <span className="uppercase tracking-wider text-[10px] text-neutral-500">
        {label}
      </span>
      <select
        value={value ?? ""}
        onChange={(e) => {
          const v = e.target.value;
          onChange(v === "" ? null : Number(v));
        }}
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

interface SimpleRow {
  url: string;
  title: string;
}

function DiffBucket({
  label,
  detail,
  count,
  rowKind,
  rows,
}: {
  label: string;
  detail: string;
  count: number;
  rowKind: "added" | "removed";
  rows: SimpleRow[];
}) {
  const tone =
    rowKind === "added"
      ? "text-diff-added bg-diff-added/10"
      : "text-diff-removed bg-diff-removed/10";

  return (
    <section>
      <header className="flex items-baseline gap-2 mb-1">
        <span className={`text-[10px] font-medium uppercase tracking-wider px-1.5 py-0.5 rounded ${tone}`}>
          {label}
        </span>
        <span className="text-xs text-neutral-300 font-medium">{count}</span>
        <span className="text-[10px] text-neutral-600">{detail}</span>
      </header>
      <ul className="border border-neutral-800 rounded divide-y divide-neutral-800">
        {rows.map((r) => (
          <li key={r.url} className="px-2 py-1.5">
            <p className="text-xs text-neutral-200 truncate">{r.title || <em className="text-neutral-600 not-italic">(untitled)</em>}</p>
            <p className="text-[10px] font-mono text-neutral-500 truncate">{r.url}</p>
          </li>
        ))}
      </ul>
    </section>
  );
}

function ModifiedBucket({
  count,
  rows,
}: {
  count: number;
  rows: { before: SimpleRow; after: SimpleRow }[];
}) {
  return (
    <section>
      <header className="flex items-baseline gap-2 mb-1">
        <span className="text-[10px] font-medium uppercase tracking-wider px-1.5 py-0.5 rounded text-amber-400 bg-amber-400/10">
          Modified
        </span>
        <span className="text-xs text-neutral-300 font-medium">{count}</span>
        <span className="text-[10px] text-neutral-600">
          Same URL, different title
        </span>
      </header>
      <ul className="border border-neutral-800 rounded divide-y divide-neutral-800">
        {rows.map((r) => (
          <li key={r.before.url} className="px-2 py-1.5 space-y-0.5">
            <p className="text-[10px] font-mono text-neutral-500 truncate">
              {r.before.url}
            </p>
            <p className="text-xs text-diff-removed line-through truncate">
              {r.before.title}
            </p>
            <p className="text-xs text-diff-added truncate">{r.after.title}</p>
          </li>
        ))}
      </ul>
    </section>
  );
}
