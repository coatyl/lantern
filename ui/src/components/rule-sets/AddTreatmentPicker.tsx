import { useEffect, useMemo, useRef, useState } from "react";
import type { TreatmentInfo } from "../../ipc/types";
import { PlusIcon } from "../Icons";
import { categoryLabel } from "./treatments";

/** "Add treatment" button with a filterable, category-grouped popover. */
export function AddTreatmentPicker({
  available,
  onAdd,
}: {
  available: TreatmentInfo[];
  onAdd: (id: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState("");
  const ref = useRef<HTMLDivElement>(null);

  const close = () => {
    setOpen(false);
    setFilter("");
  };

  // Close on a click outside the picker.
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) close();
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const groups = useMemo(() => {
    const q = filter.toLowerCase();
    const map = new Map<string, TreatmentInfo[]>();
    for (const t of available) {
      const group = categoryLabel(t.category);
      if (
        q &&
        !t.name.toLowerCase().includes(q) &&
        !t.id.toLowerCase().includes(q) &&
        !group.toLowerCase().includes(q)
      ) {
        continue;
      }
      map.set(group, [...(map.get(group) ?? []), t]);
    }
    return [...map];
  }, [available, filter]);

  if (available.length === 0) return null;

  return (
    <div ref={ref} className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        className="flex items-center gap-1.5 px-2 py-1 rounded
                   text-xs text-neutral-400 hover:text-neutral-200
                   hover:bg-surface-3 transition-colors focus:outline-none
                   focus-visible:ring-1 focus-visible:ring-accent"
      >
        <PlusIcon className="w-3 h-3" />
        Add treatment
      </button>

      {open && (
        <div
          className="absolute bottom-full left-0 mb-1 w-72 rounded
                     bg-surface-2 border border-neutral-700 shadow-xl
                     flex flex-col max-h-64 z-20"
        >
          <div className="p-2 border-b border-neutral-700">
            <input
              autoFocus
              type="text"
              placeholder="Filter treatments…"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="w-full bg-surface-3 border border-neutral-700 rounded
                         px-2 py-1 text-xs text-neutral-200 placeholder-neutral-600
                         focus:outline-none focus:ring-1 focus:ring-accent"
            />
          </div>

          <div className="overflow-y-auto flex-1">
            {groups.length === 0 ? (
              <p className="px-3 py-2 text-xs text-neutral-600">No treatments match.</p>
            ) : (
              groups.map(([group, items]) => (
                <div key={group}>
                  <p className="px-2 pt-2 pb-0.5 text-[10px] font-semibold
                                uppercase tracking-wider text-neutral-500">
                    {group}
                  </p>
                  {items.map((t) => (
                    <button
                      key={t.id}
                      onClick={() => {
                        onAdd(t.id);
                        close();
                      }}
                      className="w-full flex items-center gap-2 px-3 py-1.5
                                 text-xs text-neutral-300 hover:bg-surface-3
                                 hover:text-neutral-100 transition-colors
                                 text-left focus:outline-none
                                 focus-visible:bg-surface-3"
                    >
                      <span className="flex-1 truncate">{t.name}</span>
                      {t.destructive && (
                        <span className="text-[9px] text-danger shrink-0">destructive</span>
                      )}
                    </button>
                  ))}
                </div>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
}
