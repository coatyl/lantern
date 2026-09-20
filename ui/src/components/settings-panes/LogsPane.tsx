/**
 * Logs pane: last N entries from `<settings_dir>/logs/lantern.log`.
 *
 * Read-only.  "Refresh" re-fetches from disk.
 */

import { useCallback, useEffect, useState } from "react";
import { ipc } from "../../ipc";
import type { LogEntry, LogLevel } from "../../ipc/types";
import { SkeletonRow } from "../Skeleton";
import { EmptyState, EmptyDocumentIcon } from "../EmptyState";
import { useToast } from "../../hooks/useToast";
import { useT } from "../../i18n/I18nProvider";

function levelClass(level: LogLevel): string {
  switch (level) {
    case "error": return "text-red-400";
    case "warn":  return "text-amber-400";
    case "info":
    default:      return "text-neutral-300";
  }
}

function levelLabel(level: LogLevel): string {
  return level.toUpperCase();
}

export function LogsPane() {
  const { toast } = useToast();
  const t = useT();
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const rows = await ipc.getLogs();
      setEntries(rows);
    } catch (e) {
      // Refresh failures route through the toast surface so the rest of
      // the pane stays usable (existing entries remain visible).  v0.0.11
      // QoL slice 1.
      toast(String(e), "error");
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    load();
  }, [load]);

  return (
    <section aria-label="Application logs">
      <div className="flex items-center justify-between mb-2">
        <p className="text-[10px] font-semibold uppercase tracking-wider
                      text-neutral-500">
          Recent log entries
        </p>
        <button
          onClick={load}
          disabled={loading}
          className="text-[10px] text-neutral-500 hover:text-neutral-200
                     transition-colors focus:outline-none focus-visible:ring-1
                     focus-visible:ring-accent rounded px-1.5 py-0.5
                     border border-neutral-700 hover:border-neutral-500
                     disabled:opacity-50"
        >
          Refresh
        </button>
      </div>

      {loading ? (
        <div aria-busy="true" aria-label={t("loading.generic")}>
          {Array.from({ length: 6 }).map((_, i) => (
            <SkeletonRow key={i} />
          ))}
        </div>
      ) : entries.length === 0 ? (
        <EmptyState
          icon={<EmptyDocumentIcon />}
          title={t("empty.logs")}
          description={t("empty.logs.description")}
          ariaLabel={t("empty.logs")}
        />
      ) : (
        <div className="bg-surface-2 border border-neutral-800 rounded
                        max-h-[420px] overflow-y-auto p-2">
          {entries.map((entry, idx) => (
            <pre
              key={idx}
              className={`font-mono text-[11px] whitespace-pre-wrap leading-snug
                          ${levelClass(entry.level)}`}
            >
              {`[${entry.timestamp}] ${levelLabel(entry.level)}: ${entry.message}`}
            </pre>
          ))}
        </div>
      )}
    </section>
  );
}
