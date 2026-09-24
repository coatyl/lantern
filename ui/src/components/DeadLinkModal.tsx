import { useEffect, useMemo, useState } from "react";

import { ipc } from "../ipc";
import { useDocuments } from "../state/documents";
import type {
  LinkCheckEntry,
  LinkCheckReport,
  LinkStatus,
  TabId,
} from "../ipc/types";
import { useFocusTrap } from "../hooks/useFocusTrap";
import { useToast } from "../hooks/useToast";
import { useT } from "../i18n/I18nProvider";
import { EmptyState, EmptyCheckIcon, EmptySearchIcon } from "./EmptyState";
import { TrashIcon, XIcon } from "./Icons";

interface DeadLinkModalProps {
  open: boolean;
  tabId: TabId | null;
  tabTitle: string | null;
  onClose: () => void;
  onOpenSettings: () => void;
}

type StatusKind = LinkCheckEntry["status"]["kind"];
type SortKey = "status" | "elapsed" | "title" | "none";

const STATUS_ROWS: { key: StatusKind; label: string }[] = [
  { key: "ok", label: "OK" },
  { key: "redirect", label: "Redirect" },
  { key: "client_error", label: "4xx" },
  { key: "server_error", label: "5xx" },
  { key: "timeout", label: "Timeout" },
  { key: "network_error", label: "Network" },
  { key: "skipped", label: "Skipped" },
];

const STATUS_RANK: Record<StatusKind, number> = {
  client_error: 0,
  server_error: 1,
  network_error: 2,
  timeout: 3,
  redirect: 4,
  skipped: 5,
  ok: 6,
};

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

  // ── View state (filter + sort + selection + copy feedback) ─────────────
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
  }, [open, tabId]);

  const runCheck = async () => {
    if (tabId === null) return;
    setLoading(true);
    setError(null);
    setSelected(new Set());
    setConfirmingDelete(false);
    try {
      const next = await ipc.checkDeadLinks(tabId);
      setReport(next);
      setFilterStatus(null);
    } catch (e) {
      setReport(null);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!open || tabId === null) return;
    void runCheck();
  }, [open, tabId]);

  const counts = useMemo(() => {
    const tally = new Map<StatusKind, number>();
    for (const entry of report?.entries ?? []) {
      tally.set(entry.status.kind, (tally.get(entry.status.kind) ?? 0) + 1);
    }
    return STATUS_ROWS.map((row) => ({ ...row, count: tally.get(row.key) ?? 0 }));
  }, [report]);

  // True when a run completed and every entry came back OK.  Used to swap
  // the table for a celebratory empty state (v0.0.11 QoL slice 1).
  const allGreen =
    report !== null &&
    report.entries.length > 0 &&
    report.entries.every((entry) => entry.status.kind === "ok");

  // Filter + sort the entry list.
  const visibleEntries = useMemo(() => {
    const all = report?.entries ?? [];
    const filtered =
      filterStatus === null ? all : all.filter((e) => e.status.kind === filterStatus);

    const dir = sortDescending ? -1 : 1;
    const sorted = [...filtered];
    if (sortKey === "status") {
      sorted.sort(
        (a, b) =>
          dir * (STATUS_RANK[a.status.kind] - STATUS_RANK[b.status.kind]),
      );
    } else if (sortKey === "elapsed") {
      sorted.sort((a, b) => dir * (a.elapsed_ms - b.elapsed_ms));
    } else if (sortKey === "title") {
      sorted.sort(
        (a, b) =>
          dir * a.title.localeCompare(b.title, undefined, { sensitivity: "base" }),
      );
    }
    return sorted;
  }, [report, filterStatus, sortKey, sortDescending]);

  // ── Selection helpers (depend on visibleEntries) ────────────────────────
  const toggleSelected = (nodeId: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(nodeId)) next.delete(nodeId);
      else next.add(nodeId);
      return next;
    });
  };

  const allVisibleSelected =
    visibleEntries.length > 0 &&
    visibleEntries.every((e) => selected.has(e.node_id));

  const toggleSelectAllVisible = () => {
    setSelected((prev) => {
      if (allVisibleSelected) {
        // Clear only the visible ones; preserve selections hidden by the filter.
        const next = new Set(prev);
        for (const e of visibleEntries) next.delete(e.node_id);
        return next;
      }
      const next = new Set(prev);
      for (const e of visibleEntries) next.add(e.node_id);
      return next;
    });
  };

  const handleBulkDelete = async () => {
    if (tabId === null || selected.size === 0) return;
    setDeleting(true);
    let backgroundFailures = 0;
    try {
      // Each delete pushes its own undo entry on the source document; the
      // loop continues past individual failures so a partial success still
      // cleans up what it can.  Per-row failures are silent (most often
      // mean the row was already removed); the *background* refresh /
      // re-run failures route through the toast surface (v0.0.11 QoL
      // slice 1) so the user still hears about them.
      const ids = [...selected];
      for (const id of ids) {
        try {
          await ipc.deleteNode(tabId, id);
        } catch {
          // skip individual failures (already deleted, etc.)
        }
      }
      setConfirmingDelete(false);
      setSelected(new Set());
      try {
        await Promise.all([refreshTree(), refreshList()]);
      } catch (e) {
        backgroundFailures += 1;
        toast(String(e), "error");
      }
      try {
        await runCheck();
      } catch (e) {
        backgroundFailures += 1;
        toast(String(e), "error");
      }
    } finally {
      setDeleting(false);
      void backgroundFailures;
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
    const text = visibleEntries.map((e) => e.url).join("\n");
    try {
      await navigator.clipboard.writeText(text);
      setCopiedAt(Date.now());
    } catch (e) {
      // Clipboard unavailable (Tauri permissions, browser stub): surface
      // it as a background failure so the user doesn't wonder why nothing
      // happened.  v0.0.11 QoL slice 1.
      toast(String(e), "error");
    }
  };

  // Auto-clear "Copied" feedback after a beat.
  useEffect(() => {
    if (copiedAt === null) return;
    const t = setTimeout(() => setCopiedAt(null), 1500);
    return () => clearTimeout(t);
  }, [copiedAt]);

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
        className="bg-surface-1 border border-neutral-800 rounded-lg shadow-2xl
                   flex flex-col overflow-hidden w-[920px] max-w-[95vw] h-[620px] max-h-[90vh]"
        role="dialog"
        aria-modal="true"
        aria-label="Dead-link checker"
      >
        <div className="px-4 py-3 border-b border-neutral-800 flex items-center justify-between gap-3 shrink-0">
          <div>
            <h2 className="text-sm font-semibold text-neutral-100">Dead-link checker</h2>
            <p className="text-[10px] text-neutral-500 mt-0.5">
              {tabTitle ? `Probe every bookmark in ${tabTitle}.` : "Probe every bookmark in the active tab."}
            </p>
          </div>
          {/* audit P2 #25: hover bg + 32x32 hit area for keyboard discovery */}
          <button
            onClick={onClose}
            className="inline-flex items-center justify-center w-8 h-8 rounded
                       text-neutral-400 hover:text-neutral-100
                       hover:bg-neutral-800/60 transition-colors
                       focus:outline-none focus-visible:ring-1 focus-visible:ring-accent"
            aria-label="Close dead-link checker"
          >
            <XIcon className="w-4 h-4" />
          </button>
        </div>

        <div className="px-4 py-3 border-b border-neutral-800 flex items-center gap-3 shrink-0">
          <button
            onClick={runCheck}
            disabled={loading || tabId === null}
            className="px-3 py-1 rounded text-xs font-medium bg-accent hover:bg-accent-hover
                       text-on-accent disabled:opacity-40 transition-colors
                       focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
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
              ariaLabel={t("empty.deadlinks.preRun")}
            />
          )}

          {!report && loading && <LoadingSkeleton />}

          {report && (
            <>
              {/* Summary cards.  Status cards become toggle filters. */}
              <div className="grid grid-cols-2 md:grid-cols-4 xl:grid-cols-7 gap-2">
                <SummaryCard label="Bookmarks" value={String(report.total_bookmarks)} />
                <SummaryCard label="Probed" value={String(report.probed)} />
                {counts.map((count) => (
                  <SummaryCard
                    key={count.key}
                    label={count.label}
                    value={String(count.count)}
                    active={filterStatus === count.key}
                    disabled={count.count === 0}
                    tone={count.key}
                    onClick={
                      count.count === 0
                        ? undefined
                        : () =>
                            setFilterStatus((prev) => (prev === count.key ? null : count.key))
                    }
                  />
                ))}
              </div>

              {/* Action row */}
              <div className="flex items-center gap-2 text-[10px] text-neutral-500">
                <span>
                  {filterStatus
                    ? `Showing ${visibleEntries.length} of ${report.entries.length} (filter: ${labelForKind(filterStatus)})`
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
                {selected.size > 0 && (
                  <span className="text-accent">
                    {selected.size} selected
                  </span>
                )}
                <div className="ml-auto flex items-center gap-2">
                  {copiedAt !== null && (
                    <span className="text-diff-added">Copied ✓</span>
                  )}
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

              {/* Bulk-delete confirmation strip */}
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

              {/* All-green celebratory empty state: only when every entry
                  came back OK and no filter is hiding rows.  v0.0.11 QoL
                  slice 1. */}
              {allGreen && filterStatus === null ? (
                <EmptyState
                  icon={<EmptyCheckIcon />}
                  title={t("empty.deadlinks.allGreen", { count: report.entries.length })}
                  ariaLabel={t("empty.deadlinks.allGreen", { count: report.entries.length })}
                />
              ) : (
                /* Results table */
                <div className="border border-neutral-800 rounded overflow-hidden">
                  <div className="grid grid-cols-[28px_110px_90px_1fr] gap-3 px-3 py-2 bg-surface-2 text-[10px] uppercase tracking-wider text-neutral-500 items-center">
                    <input
                      type="checkbox"
                      aria-label="Select all visible entries"
                      checked={allVisibleSelected}
                      onChange={toggleSelectAllVisible}
                      disabled={visibleEntries.length === 0}
                    />
                    <SortHeader label="Status"   active={sortKey === "status"}   descending={sortDescending} onClick={() => handleSortClick("status")}   />
                    <SortHeader label="Time"     active={sortKey === "elapsed"}  descending={sortDescending} onClick={() => handleSortClick("elapsed")}  />
                    <SortHeader label="Bookmark" active={sortKey === "title"}    descending={sortDescending} onClick={() => handleSortClick("title")}    />
                  </div>
                  <ul className="divide-y divide-neutral-800">
                    {visibleEntries.length === 0 && (
                      <li className="px-3 py-4 text-[11px] text-neutral-600 italic text-center">
                        No entries match the current filter.
                      </li>
                    )}
                    {visibleEntries.map((entry) => (
                      // audit P2 #24: selected-row tint bumped from bg-accent/5
                      // (~1.05:1 lift) to bg-accent/15 so the selected state is
                      // perceivable without leaning on the checkbox alone.
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
                        <StatusBadge status={entry.status} />
                        <span className="text-[10px] text-neutral-500 tabular-nums">
                          {formatElapsed(entry.elapsed_ms)}
                        </span>
                        <div className="min-w-0">
                          <p className="text-xs text-neutral-200 truncate">
                            {entry.title || <em className="text-neutral-600 not-italic">(untitled)</em>}
                          </p>
                          <p className="text-[10px] font-mono text-neutral-500 break-all">
                            {entry.url}
                          </p>
                        </div>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}

interface SummaryCardProps {
  label: string;
  value: string;
  active?: boolean;
  disabled?: boolean;
  tone?: StatusKind;
  onClick?: () => void;
}

function SummaryCard({ label, value, active, disabled, tone, onClick }: SummaryCardProps) {
  const interactive = onClick !== undefined;
  // audit P2 #24: border default bumped from neutral-800 (~2:1) to neutral-700
  // (~3.5:1) so non-active cards still register as cards; active state now layers
  // a tone-tinted background (active && tone ? bg-surface-3 : bg-surface-2) on
  // top of the toneBorderClass so the toggled filter is unambiguous.
  const accentBorder = active && tone ? toneBorderClass(tone) : "border-neutral-700";
  const activeBg = active ? "bg-surface-3" : "bg-surface-2";
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled || !interactive}
      aria-pressed={active}
      className={`text-left rounded border ${activeBg} px-3 py-2
                  transition-colors
                  ${accentBorder}
                  ${interactive
                    ? "hover:border-neutral-500 focus:outline-none focus-visible:ring-1 focus-visible:ring-accent cursor-pointer"
                    : "cursor-default"}
                  ${disabled ? "opacity-50 cursor-not-allowed" : ""}`}
    >
      {/* audit P2 #21: label neutral-500 (~3.7:1) → neutral-400 (~5.5:1). */}
      <div className="text-[10px] uppercase tracking-wider text-neutral-400">{label}</div>
      <div className="mt-1 text-sm font-semibold text-neutral-100 tabular-nums">{value}</div>
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
      {/* Card grid skeleton */}
      <div className="grid grid-cols-2 md:grid-cols-4 xl:grid-cols-7 gap-2">
        {Array.from({ length: 7 }).map((_, i) => (
          <div
            key={i}
            className="rounded border border-neutral-800 bg-surface-2 px-3 py-2 h-12"
          />
        ))}
      </div>
      {/* Table skeleton */}
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

function StatusBadge({ status }: { status: LinkStatus }) {
  const tone = statusTone(status.kind);
  return (
    <span className={`inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider ${tone}`}>
      {statusLabel(status)}
    </span>
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

function labelForKind(kind: StatusKind): string {
  return STATUS_ROWS.find((r) => r.key === kind)?.label ?? kind;
}

function statusTone(kind: StatusKind): string {
  switch (kind) {
    case "ok":
      return "bg-diff-added/10 text-diff-added";
    case "redirect":
      return "bg-accent/10 text-accent";
    case "client_error":
    case "server_error":
      return "bg-diff-removed/10 text-diff-removed";
    case "timeout":
    case "network_error":
      return "bg-danger/10 text-danger";
    case "skipped":
      return "bg-neutral-800 text-neutral-400";
  }
}

function toneBorderClass(kind: StatusKind): string {
  switch (kind) {
    case "ok":
      return "border-diff-added";
    case "redirect":
      return "border-accent";
    case "client_error":
    case "server_error":
      return "border-diff-removed";
    case "timeout":
    case "network_error":
      return "border-danger";
    case "skipped":
      return "border-neutral-500";
  }
}

function formatElapsed(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(2)}s`;
  return `${ms}ms`;
}
