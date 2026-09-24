/**
 * Dead-link checker (Tools → Check dead links…): probes every bookmark in the
 * active tab, summarises the results by status, and lets the user filter,
 * sort, copy, and bulk-delete entries. Network access stays opt-in; the
 * backend refuses the check until it is enabled in Settings.
 */

import { useEffect, useMemo, useState } from "react";

import { ipc } from "../ipc";
import { useDocuments } from "../state/documents";
import type { LinkCheckEntry, LinkCheckReport, LinkStatus, TabId } from "../ipc/types";
import { useToast } from "../hooks/useToast";
import { useT } from "../i18n/I18nProvider";
import { EmptyState, EmptyCheckIcon, EmptySearchIcon } from "./EmptyState";
import { TrashIcon } from "./Icons";
import { Modal, ModalHeader, primaryButton } from "./Modal";

interface DeadLinkModalProps {
  open: boolean;
  tabId: TabId | null;
  tabTitle: string | null;
  onClose: () => void;
  onOpenSettings: () => void;
}

type StatusKind = LinkStatus["kind"];
type SortKey = "status" | "elapsed" | "title";

/**
 * Per-status presentation. `rank` orders the "status" sort (worst first);
 * `badge` styles the row badge and `border` the active summary card.
 */
const STATUS: Record<StatusKind, { label: string; rank: number; badge: string; border: string }> = {
  ok:            { label: "OK",       rank: 6, badge: "bg-diff-added/10 text-diff-added",     border: "border-diff-added" },
  redirect:      { label: "Redirect", rank: 4, badge: "bg-accent/10 text-accent",             border: "border-accent" },
  client_error:  { label: "4xx",      rank: 0, badge: "bg-diff-removed/10 text-diff-removed", border: "border-diff-removed" },
  server_error:  { label: "5xx",      rank: 1, badge: "bg-diff-removed/10 text-diff-removed", border: "border-diff-removed" },
  timeout:       { label: "Timeout",  rank: 3, badge: "bg-danger/10 text-danger",             border: "border-danger" },
  network_error: { label: "Network",  rank: 2, badge: "bg-danger/10 text-danger",             border: "border-danger" },
  skipped:       { label: "Skipped",  rank: 5, badge: "bg-neutral-800 text-neutral-400",      border: "border-neutral-500" },
};

/** Summary-card order. */
const STATUS_KINDS = Object.keys(STATUS) as StatusKind[];

export function DeadLinkModal({
  open,
  tabId,
  tabTitle,
  onClose,
  onOpenSettings,
}: DeadLinkModalProps) {
  const [report, setReport] = useState<LinkCheckReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [filterStatus, setFilterStatus] = useState<StatusKind | null>(null);
  const [sortKey, setSortKey] = useState<SortKey>("status");
  const [sortDescending, setSortDescending] = useState(false);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [copiedAt, setCopiedAt] = useState<number | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [confirmingDelete, setConfirmingDelete] = useState(false);

  const refreshTree = useDocuments((s) => s.refreshTree);
  const refreshList = useDocuments((s) => s.refreshList);
  const { toast } = useToast();
  const t = useT();

  // Never throws: failures land in `error`.
  const runCheck = async () => {
    if (tabId === null) return;
    setLoading(true);
    setError(null);
    setSelected(new Set());
    setConfirmingDelete(false);
    try {
      setReport(await ipc.checkDeadLinks(tabId));
      setFilterStatus(null);
    } catch (e) {
      setReport(null);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  // Start from a clean slate and run a check whenever the modal opens or the
  // target tab changes.
  useEffect(() => {
    if (!open) return;
    setReport(null);
    setError(null);
    setLoading(false);
    setFilterStatus(null);
    setSortKey("status");
    setSortDescending(false);
    setSelected(new Set());
    setCopiedAt(null);
    setConfirmingDelete(false);
    void runCheck();
  }, [open, tabId]);

  // Auto-clear the "Copied" feedback after a beat.
  useEffect(() => {
    if (copiedAt === null) return;
    const timer = setTimeout(() => setCopiedAt(null), 1500);
    return () => clearTimeout(timer);
  }, [copiedAt]);

  const counts = useMemo(() => {
    const tally = new Map<StatusKind, number>();
    for (const entry of report?.entries ?? []) {
      tally.set(entry.status.kind, (tally.get(entry.status.kind) ?? 0) + 1);
    }
    return STATUS_KINDS.map((kind) => ({ kind, count: tally.get(kind) ?? 0 }));
  }, [report]);

  const visibleEntries = useMemo(() => {
    const all = report?.entries ?? [];
    const filtered = filterStatus === null ? all : all.filter((e) => e.status.kind === filterStatus);
    const dir = sortDescending ? -1 : 1;
    const compare: Record<SortKey, (a: LinkCheckEntry, b: LinkCheckEntry) => number> = {
      status: (a, b) => STATUS[a.status.kind].rank - STATUS[b.status.kind].rank,
      elapsed: (a, b) => a.elapsed_ms - b.elapsed_ms,
      title: (a, b) => a.title.localeCompare(b.title, undefined, { sensitivity: "base" }),
    };
    return [...filtered].sort((a, b) => dir * compare[sortKey](a, b));
  }, [report, filterStatus, sortKey, sortDescending]);

  if (!open) return null;

  // A completed run where every entry came back OK swaps the table for a
  // celebratory empty state.
  const allGreen =
    report !== null &&
    report.entries.length > 0 &&
    report.entries.every((entry) => entry.status.kind === "ok");

  const toggleSelected = (nodeId: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(nodeId)) next.delete(nodeId);
      else next.add(nodeId);
      return next;
    });
  };

  const allVisibleSelected =
    visibleEntries.length > 0 && visibleEntries.every((e) => selected.has(e.node_id));

  // Only touches visible rows; selections hidden by the filter are kept.
  const toggleSelectAllVisible = () => {
    setSelected((prev) => {
      const next = new Set(prev);
      for (const e of visibleEntries) {
        if (allVisibleSelected) next.delete(e.node_id);
        else next.add(e.node_id);
      }
      return next;
    });
  };

  const handleBulkDelete = async () => {
    if (tabId === null || selected.size === 0) return;
    setDeleting(true);
    try {
      // Each delete pushes its own undo entry on the source document. A row
      // that fails (most often: already removed) is skipped silently so the
      // rest still go through.
      for (const id of selected) {
        try {
          await ipc.deleteNode(tabId, id);
        } catch {
          // skip
        }
      }
      setConfirmingDelete(false);
      setSelected(new Set());
      try {
        await Promise.all([refreshTree(), refreshList()]);
      } catch (e) {
        toast(String(e), "error");
      }
      await runCheck();
    } finally {
      setDeleting(false);
    }
  };

  const handleSortClick = (key: SortKey) => {
    if (sortKey === key) {
      setSortDescending((d) => !d);
    } else {
      setSortKey(key);
      setSortDescending(false);
    }
  };

  const copyVisibleUrls = async () => {
    if (visibleEntries.length === 0) return;
    try {
      await navigator.clipboard.writeText(visibleEntries.map((e) => e.url).join("\n"));
      setCopiedAt(Date.now());
    } catch (e) {
      // Clipboard unavailable (permissions, browser stub).
      toast(String(e), "error");
    }
  };

  return (
    <Modal
      label="Dead-link checker"
      onClose={onClose}
      className="flex flex-col w-[920px] h-[620px] max-h-[90vh]"
    >
      <ModalHeader
        title="Dead-link checker"
        subtitle={
          tabTitle
            ? `Probe every bookmark in ${tabTitle}.`
            : "Probe every bookmark in the active tab."
        }
        closeLabel="Close dead-link checker"
        onClose={onClose}
      />

      <div className="px-4 py-3 border-b border-neutral-800 flex items-center gap-3 shrink-0">
        <button
          onClick={runCheck}
          disabled={loading || tabId === null}
          className={`px-3 py-1 ${primaryButton}`}
        >
          {loading ? "Checking..." : "Run check"}
        </button>
        <p className="text-[10px] text-neutral-600">
          Network requests stay opt-in. If disabled, enable the checker in Settings first.
        </p>
        {error && error.toLowerCase().includes("disabled") && (
          <button
            onClick={onOpenSettings}
            className="ml-auto px-2 py-1 rounded border border-neutral-700 text-[10px]
                       text-neutral-300 hover:border-neutral-500 transition-colors
                       focus:outline-none focus-visible:ring-1 focus-visible:ring-accent"
          >
            Open Settings
          </button>
        )}
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto px-4 py-3 space-y-4">
        {error && (
          <div className="rounded border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger">
            {error}
          </div>
        )}

        {!report && !loading && !error && (
          <EmptyState
            icon={<EmptySearchIcon />}
            title={t("empty.deadlinks.preRun")}
            description="Run the checker to classify URLs as OK, redirect, error, timeout, network error, or skipped. It stays off until you ask."
          />
        )}

        {!report && loading && <LoadingSkeleton />}

        {report && (
          <>
            {/* Summary cards; the status cards double as toggle filters. */}
            <div className="grid grid-cols-2 md:grid-cols-4 xl:grid-cols-7 gap-2">
              <SummaryCard label="Bookmarks" value={report.total_bookmarks} />
              <SummaryCard label="Probed" value={report.probed} />
              {counts.map(({ kind, count }) => (
                <SummaryCard
                  key={kind}
                  label={STATUS[kind].label}
                  value={count}
                  active={filterStatus === kind}
                  activeBorder={STATUS[kind].border}
                  onClick={
                    count === 0
                      ? undefined
                      : () => setFilterStatus((prev) => (prev === kind ? null : kind))
                  }
                />
              ))}
            </div>

            {/* Action row */}
            <div className="flex items-center gap-2 text-[10px] text-neutral-500">
              <span>
                {filterStatus
                  ? `Showing ${visibleEntries.length} of ${report.entries.length} (filter: ${STATUS[filterStatus].label})`
                  : `Showing ${visibleEntries.length} entries`}
              </span>
              {filterStatus !== null && (
                <button
                  onClick={() => setFilterStatus(null)}
                  className="text-neutral-400 hover:text-neutral-200 underline-offset-2 hover:underline"
                >
                  clear
                </button>
              )}
              {selected.size > 0 && <span className="text-accent">{selected.size} selected</span>}
              <div className="ml-auto flex items-center gap-2">
                {copiedAt !== null && <span className="text-diff-added">Copied ✓</span>}
                <button
                  onClick={copyVisibleUrls}
                  disabled={visibleEntries.length === 0}
                  className="px-2 py-0.5 rounded border border-neutral-700 text-neutral-300
                             hover:border-neutral-500 hover:text-neutral-100
                             disabled:opacity-40 transition-colors
                             focus:outline-none focus-visible:ring-1 focus-visible:ring-accent"
                >
                  Copy URLs
                </button>
                {selected.size > 0 && !confirmingDelete && (
                  <button
                    onClick={() => setConfirmingDelete(true)}
                    className="inline-flex items-center gap-1 px-2 py-0.5 rounded
                               border border-danger/40 text-danger
                               hover:border-danger hover:bg-danger/10
                               transition-colors focus:outline-none
                               focus-visible:ring-1 focus-visible:ring-danger"
                  >
                    <TrashIcon className="w-3 h-3" />
                    Delete {selected.size}
                  </button>
                )}
              </div>
            </div>

            {confirmingDelete && (
              <div className="rounded border border-danger/30 bg-danger/10 px-3 py-2
                              flex items-center gap-3 text-[11px] text-danger">
                <span>
                  Delete <strong>{selected.size}</strong> bookmark{selected.size === 1 ? "" : "s"}?
                  Each removal is independently undoable from the source document.
                </span>
                <div className="ml-auto flex items-center gap-2">
                  <button
                    onClick={() => setConfirmingDelete(false)}
                    disabled={deleting}
                    className="px-2 py-0.5 rounded border border-neutral-700 text-neutral-300
                               hover:border-neutral-500 transition-colors
                               disabled:opacity-40 focus:outline-none"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleBulkDelete}
                    disabled={deleting}
                    className="px-2 py-0.5 rounded bg-danger text-on-danger
                               hover:bg-danger/80 transition-colors
                               disabled:opacity-40 focus:outline-none"
                  >
                    {deleting ? "Deleting…" : `Delete ${selected.size}`}
                  </button>
                </div>
              </div>
            )}

            {/* The all-green state only replaces the table when no filter hides rows. */}
            {allGreen && filterStatus === null ? (
              <EmptyState
                icon={<EmptyCheckIcon />}
                title={t("empty.deadlinks.allGreen", { count: report.entries.length })}
              />
            ) : (
              <div className="border border-neutral-800 rounded overflow-hidden">
                <div className="grid grid-cols-[28px_110px_90px_1fr] gap-3 px-3 py-2 bg-surface-2 text-[10px] uppercase tracking-wider text-neutral-500 items-center">
                  <input
                    type="checkbox"
                    aria-label="Select all visible entries"
                    checked={allVisibleSelected}
                    onChange={toggleSelectAllVisible}
                    disabled={visibleEntries.length === 0}
                  />
                  <SortHeader label="Status"   active={sortKey === "status"}  descending={sortDescending} onClick={() => handleSortClick("status")} />
                  <SortHeader label="Time"     active={sortKey === "elapsed"} descending={sortDescending} onClick={() => handleSortClick("elapsed")} />
                  <SortHeader label="Bookmark" active={sortKey === "title"}   descending={sortDescending} onClick={() => handleSortClick("title")} />
                </div>
                <ul className="divide-y divide-neutral-800">
                  {visibleEntries.length === 0 && (
                    <li className="px-3 py-4 text-[11px] text-neutral-600 italic text-center">
                      No entries match the current filter.
                    </li>
                  )}
                  {visibleEntries.map((entry) => (
                    <li
                      key={entry.node_id}
                      className={`grid grid-cols-[28px_110px_90px_1fr] gap-3 px-3 py-2 items-start
                                  ${selected.has(entry.node_id) ? "bg-accent/15" : ""}`}
                    >
                      <input
                        type="checkbox"
                        aria-label={`Select ${entry.title || entry.url}`}
                        checked={selected.has(entry.node_id)}
                        onChange={() => toggleSelected(entry.node_id)}
                        className="mt-0.5"
                      />
                      <span
                        className={`inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider ${STATUS[entry.status.kind].badge}`}
                      >
                        {statusLabel(entry.status)}
                      </span>
                      <span className="text-[10px] text-neutral-500 tabular-nums">
                        {formatElapsed(entry.elapsed_ms)}
                      </span>
                      <div className="min-w-0">
                        <p className="text-xs text-neutral-200 truncate">
                          {entry.title || <em className="text-neutral-600 not-italic">(untitled)</em>}
                        </p>
                        <p className="text-[10px] font-mono text-neutral-500 break-all">{entry.url}</p>
                      </div>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </>
        )}
      </div>
    </Modal>
  );
}

const CARD_CLASS = "text-left rounded border px-3 py-2 transition-colors";

function CardContent({ label, value }: { label: string; value: number }) {
  return (
    <>
      <div className="text-[10px] uppercase tracking-wider text-neutral-400">{label}</div>
      <div className="mt-1 text-sm font-semibold text-neutral-100 tabular-nums">{value}</div>
    </>
  );
}

/**
 * A summary count. With `onClick` it is a toggle button (a status filter);
 * without, a static tile. A status with no entries renders as a dimmed,
 * disabled toggle.
 */
function SummaryCard({
  label,
  value,
  active = false,
  activeBorder,
  onClick,
}: {
  label: string;
  value: number;
  active?: boolean;
  activeBorder?: string;
  onClick?: () => void;
}) {
  if (activeBorder === undefined) {
    return (
      <div className={`${CARD_CLASS} bg-surface-2 border-neutral-700 cursor-default`}>
        <CardContent label={label} value={value} />
      </div>
    );
  }
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={!onClick}
      aria-pressed={active}
      className={`${CARD_CLASS}
                  ${active ? `bg-surface-3 ${activeBorder}` : "bg-surface-2 border-neutral-700"}
                  ${onClick
                    ? "hover:border-neutral-500 focus:outline-none focus-visible:ring-1 focus-visible:ring-accent cursor-pointer"
                    : "opacity-50 cursor-not-allowed"}`}
    >
      <CardContent label={label} value={value} />
    </button>
  );
}

function SortHeader({
  label,
  active,
  descending,
  onClick,
}: {
  label: string;
  active: boolean;
  descending: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`text-left flex items-center gap-1 transition-colors
                  ${active ? "text-neutral-200" : "hover:text-neutral-300"}
                  focus:outline-none focus-visible:underline`}
    >
      <span>{label}</span>
      {active && (
        <span className="text-[8px]" aria-hidden>
          {descending ? "▼" : "▲"}
        </span>
      )}
    </button>
  );
}

function LoadingSkeleton() {
  return (
    <div className="space-y-4 animate-pulse" aria-busy="true" aria-live="polite">
      <div className="grid grid-cols-2 md:grid-cols-4 xl:grid-cols-7 gap-2">
        {Array.from({ length: 7 }).map((_, i) => (
          <div key={i} className="rounded border border-neutral-800 bg-surface-2 px-3 py-2 h-12" />
        ))}
      </div>
      <div className="border border-neutral-800 rounded overflow-hidden">
        <div className="h-7 bg-surface-2 border-b border-neutral-800" />
        {Array.from({ length: 5 }).map((_, i) => (
          <div
            key={i}
            className="h-10 border-b border-neutral-800 last:border-b-0
                       grid grid-cols-[110px_90px_1fr] gap-3 px-3 py-2 items-center"
          >
            <div className="h-3 w-16 rounded bg-neutral-800" />
            <div className="h-3 w-12 rounded bg-neutral-800" />
            <div className="h-3 w-3/4 rounded bg-neutral-800" />
          </div>
        ))}
      </div>
    </div>
  );
}

function statusLabel(status: LinkStatus): string {
  switch (status.kind) {
    case "ok":
    case "redirect":
    case "client_error":
    case "server_error":
      return `${status.kind.replace("_", " ")} ${status.code}`;
    case "network_error":
      return "network error";
    case "timeout":
      return "timeout";
    case "skipped":
      return `skipped ${status.reason.replace("_", " ")}`;
  }
}

function formatElapsed(ms: number): string {
  return ms >= 1000 ? `${(ms / 1000).toFixed(2)}s` : `${ms}ms`;
}
