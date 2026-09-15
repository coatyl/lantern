/**
 * Keyboard shortcuts pane: read-only table grouped by category.
 *
 * Backend feeds the canonical list via `list_shortcuts`; rebinding is
 * deferred to a future release (PRD §8.9).
 */

import { useEffect, useState } from "react";
import { ipc } from "../../ipc";
import type { ShortcutBinding } from "../../ipc/types";
import { SkeletonText } from "../Skeleton";
import { useToast } from "../../hooks/useToast";
import { useT } from "../../i18n/I18nProvider";

export function KeyboardPane() {
  const { toast } = useToast();
  const t = useT();
  const [shortcuts, setShortcuts] = useState<ShortcutBinding[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    ipc.listShortcuts()
      .then((rows) => {
        if (!cancelled) setShortcuts(rows);
      })
      .catch((e) => {
        if (!cancelled) {
          // Load failures route through the toast surface so the user sees
          // the issue without losing the footer hint.  v0.0.11 QoL slice 1.
          toast(String(e), "error");
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [toast]);

  // Group by category, preserving the order in which categories first appear.
  const grouped = (() => {
    const order: string[] = [];
    const map = new Map<string, ShortcutBinding[]>();
    for (const row of shortcuts) {
      if (!map.has(row.category)) {
        order.push(row.category);
        map.set(row.category, []);
      }
      map.get(row.category)!.push(row);
    }
    return order.map((cat) => ({ category: cat, rows: map.get(cat)! }));
  })();

  return (
    <section aria-label="Keyboard shortcuts">
      {loading ? (
        <div aria-busy="true" aria-label={t("loading.generic")} className="space-y-2">
          {Array.from({ length: 8 }).map((_, i) => (
            <SkeletonText key={i} lines={1} />
          ))}
        </div>
      ) : shortcuts.length === 0 ? (
        <p className="text-xs text-neutral-600">(no shortcuts registered)</p>
      ) : (
        <div className="space-y-4">
          {grouped.map(({ category, rows }) => (
            <div key={category}>
              <p className="text-[10px] font-semibold uppercase tracking-wider
                            text-neutral-500 mb-1.5">
                {category}
              </p>
              <table className="w-full text-xs">
                <thead>
                  <tr className="text-left text-neutral-600">
                    <th className="font-normal pb-1 pr-4">Action</th>
                    <th className="font-normal pb-1">Shortcut</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((row) => (
                    <tr key={row.action_id} className="border-t border-neutral-800">
                      <td className="py-1.5 pr-4 text-neutral-300">{row.label}</td>
                      <td className="py-1.5">
                        <kbd className="font-mono text-[11px] text-neutral-200
                                        bg-surface-2 border border-neutral-700
                                        rounded px-1.5 py-0.5">
                          {row.key_combo}
                        </kbd>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ))}
        </div>
      )}

      <p className="mt-4 text-[10px] text-neutral-700 leading-snug">
        Rebinding will arrive in a future release.
      </p>
    </section>
  );
}
