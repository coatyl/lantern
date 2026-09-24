/**
 * Structured filter drawer (v0.0.5).
 *
 * Sections: kind / date range / domain allowlist / TLD allowlist / URL
 * scheme / folder depth.  Composes with logical AND.
 *
 * State lives in the documents store; this component is a controlled view
 * over that state plus a couple of small input-buffer pieces that should
 * not leak to the store (the partially-typed text in chip inputs).
 */

import { useState, type KeyboardEvent as ReactKE } from "react";

import type {
  FilterSpec,
  ItemKind,
  DateRange,
  DepthFilter,
} from "../ipc/types";
import { XIcon } from "./Icons";

// ─── Helpers ──────────────────────────────────────────────────────────────

/** Convert "" to null; trim and lowercase non-empty values. */
function normalize(value: string): string | null {
  const v = value.trim().toLowerCase();
  return v.length ? v : null;
}

/** Parse "YYYY-MM-DD" into Unix-seconds (start of day, local TZ). */
function parseDate(value: string): number | null {
  if (!value) return null;
  const ms = Date.parse(value);
  return Number.isNaN(ms) ? null : Math.floor(ms / 1000);
}

/** Render a Unix-seconds value as "YYYY-MM-DD" for an `<input type="date">`. */
function formatDate(seconds: number | null | undefined): string {
  if (seconds == null) return "";
  return new Date(seconds * 1000).toISOString().slice(0, 10);
}

// ─── Component ────────────────────────────────────────────────────────────

interface FilterDrawerProps {
  filter: FilterSpec | null;
  onChange: (next: FilterSpec | null) => void;
  /** Hide the depth section when irrelevant (browse mode). */
  showDepth: boolean;
  /** DOM id for the outer element, used by `aria-controls` on the toggle. */
  id?: string;
}

const ALL_KINDS: ItemKind[] = ["bookmark", "folder", "separator"];
const COMMON_SCHEMES = ["https", "http", "ftp", "file"] as const;

export function FilterDrawer({ filter, onChange, showDepth, id }: FilterDrawerProps) {
  // Mutate-and-emit helper.
  const update = (partial: Partial<FilterSpec>) => {
    onChange({ ...(filter ?? {}), ...partial });
  };

  // ── Kind ─────────────────────────────────────────────────────────────────
  const activeKinds = filter?.kinds ?? null;
  const kindActive = (k: ItemKind) =>
    activeKinds === null || activeKinds.length === 0 || activeKinds.includes(k);
  const toggleKind = (k: ItemKind) => {
    const current = activeKinds ?? [...ALL_KINDS];
    const next = current.includes(k)
      ? current.filter((x) => x !== k)
      : [...current, k];
    // All three on = no constraint.  Treat as `null` so the badge clears.
    update({
      kinds: next.length === 0 || next.length === ALL_KINDS.length ? null : next,
    });
  };

  // ── Date range ───────────────────────────────────────────────────────────
  const dateRange: DateRange | null = filter?.date_range ?? null;
  const setDateRange = (next: Partial<DateRange>) => {
    const merged = { ...(dateRange ?? {}), ...next };
    const empty = merged.since == null && merged.until == null;
    update({ date_range: empty ? null : merged });
  };

  // ── Schemes ──────────────────────────────────────────────────────────────
  const activeSchemes = filter?.schemes ?? null;
  const schemeActive = (s: string) =>
    activeSchemes === null || activeSchemes.includes(s);
  const toggleScheme = (s: string) => {
    const current = activeSchemes ?? [];
    const next = current.includes(s)
      ? current.filter((x) => x !== s)
      : [...current, s];
    update({ schemes: next.length === 0 ? null : next });
  };

  // ── Depth ────────────────────────────────────────────────────────────────
  const depth: DepthFilter | null = filter?.depth ?? null;
  const setDepth = (next: Partial<DepthFilter>) => {
    const merged = { ...(depth ?? {}), ...next };
    const empty = merged.min == null && merged.max == null;
    update({ depth: empty ? null : merged });
  };

  return (
    <div
      id={id}
      className="flex flex-col gap-2 px-3 py-2 border-b border-neutral-800
                 bg-surface-2 shrink-0 text-[11px] text-neutral-300"
    >
      {/* Kind */}
      <FilterRow label="Kind">
        {ALL_KINDS.map((k) => (
          <label
            key={k}
            className="flex items-center gap-1 cursor-pointer select-none"
          >
            <input
              type="checkbox"
              checked={kindActive(k)}
              onChange={() => toggleKind(k)}
            />
            <span className="capitalize text-neutral-400">{k}s</span>
          </label>
        ))}
      </FilterRow>

      {/* Date range (audit P2 #22): input borders neutral-700 → neutral-600 so
          they cross the 3:1 UI-component contrast floor against surface-1. */}
      <FilterRow label="Added">
        <input
          type="date"
          value={formatDate(dateRange?.since)}
          onChange={(e) => setDateRange({ since: parseDate(e.target.value) })}
          className="bg-surface-1 border border-neutral-600 rounded px-1.5 py-0.5
                     text-[10px] text-neutral-200
                     focus:outline-none focus-visible:border-accent"
        />
        <span className="text-neutral-600">→</span>
        <input
          type="date"
          value={formatDate(dateRange?.until)}
          onChange={(e) => setDateRange({ until: parseDate(e.target.value) })}
          className="bg-surface-1 border border-neutral-600 rounded px-1.5 py-0.5
                     text-[10px] text-neutral-200
                     focus:outline-none focus-visible:border-accent"
        />
      </FilterRow>

      {/* Domain allowlist */}
      <FilterRow label="Domain">
        <ChipInput
          values={filter?.domains ?? []}
          placeholder="example.com, news.ycombinator.com…"
          onAdd={(v) => {
            const value = normalize(v);
            if (value === null) return;
            const list = filter?.domains ?? [];
            if (list.includes(value)) return;
            update({ domains: [...list, value] });
          }}
          onRemove={(v) => {
            const list = (filter?.domains ?? []).filter((x) => x !== v);
            update({ domains: list.length === 0 ? null : list });
          }}
        />
      </FilterRow>

      {/* TLD allowlist */}
      <FilterRow label="TLD">
        <ChipInput
          values={filter?.tlds ?? []}
          placeholder="com, org, dev…"
          onAdd={(v) => {
            const value = normalize(v.replace(/^\./, ""));
            if (value === null) return;
            const list = filter?.tlds ?? [];
            if (list.includes(value)) return;
            update({ tlds: [...list, value] });
          }}
          onRemove={(v) => {
            const list = (filter?.tlds ?? []).filter((x) => x !== v);
            update({ tlds: list.length === 0 ? null : list });
          }}
        />
      </FilterRow>

      {/* URL schemes */}
      <FilterRow label="Scheme">
        {COMMON_SCHEMES.map((s) => (
          <label key={s} className="flex items-center gap-1 cursor-pointer select-none">
            <input
              type="checkbox"
              checked={schemeActive(s)}
              onChange={() => toggleScheme(s)}
            />
            <span className="text-neutral-400 font-mono text-[10px]">{s}</span>
          </label>
        ))}
      </FilterRow>

      {/* Depth: only meaningful in search mode */}
      {showDepth && (
        <FilterRow label="Depth">
          <input
            type="number"
            min={0}
            value={depth?.min ?? ""}
            onChange={(e) =>
              setDepth({ min: e.target.value === "" ? null : Number(e.target.value) })
            }
            className="w-14 bg-surface-1 border border-neutral-600 rounded px-1.5 py-0.5
                       text-[10px] text-neutral-200 text-right
                       focus:outline-none focus-visible:border-accent"
            placeholder="min"
          />
          <span className="text-neutral-600">-</span>
          <input
            type="number"
            min={0}
            value={depth?.max ?? ""}
            onChange={(e) =>
              setDepth({ max: e.target.value === "" ? null : Number(e.target.value) })
            }
            className="w-14 bg-surface-1 border border-neutral-600 rounded px-1.5 py-0.5
                       text-[10px] text-neutral-200 text-right
                       focus:outline-none focus-visible:border-accent"
            placeholder="max"
          />
          <span className="text-[10px] text-neutral-600">levels deep</span>
        </FilterRow>
      )}

      {/* Reset row */}
      {filter !== null && (
        <div className="flex justify-end pt-0.5">
          <button
            type="button"
            onClick={() => onChange(null)}
            className="text-[10px] text-neutral-500 hover:text-neutral-200
                       transition-colors focus:outline-none focus-visible:underline"
          >
            Reset all filters
          </button>
        </div>
      )}
    </div>
  );
}

// ─── Subcomponents ────────────────────────────────────────────────────────

function FilterRow({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center gap-2 flex-wrap">
      <span className="w-14 shrink-0 text-[10px] uppercase tracking-wider text-neutral-600">
        {label}
      </span>
      {children}
    </div>
  );
}

interface ChipInputProps {
  values: string[];
  placeholder: string;
  onAdd: (raw: string) => void;
  onRemove: (value: string) => void;
}

function ChipInput({ values, placeholder, onAdd, onRemove }: ChipInputProps) {
  const [draft, setDraft] = useState("");

  const commit = () => {
    if (draft.trim().length === 0) return;
    // Allow comma-separated bulk paste.
    for (const piece of draft.split(/[,\s]+/)) {
      if (piece) onAdd(piece);
    }
    setDraft("");
  };

  const onKeyDown = (e: ReactKE<HTMLInputElement>) => {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      commit();
    } else if (e.key === "Backspace" && draft === "" && values.length > 0) {
      onRemove(values[values.length - 1]);
    }
  };

  return (
    <div
      className="flex items-center gap-1 flex-wrap flex-1 min-w-0
                 bg-surface-1 border border-neutral-600 rounded px-1.5 py-0.5
                 focus-within:border-accent transition-colors"
    >
      {values.map((v) => (
        <span
          key={v}
          className="inline-flex items-center gap-1 rounded bg-surface-2
                     border border-neutral-600 px-1.5 py-0.5 text-[10px]
                     text-neutral-200 font-mono"
        >
          {v}
          {/* audit P2 #21/missing focus ring: chip remove button gains a
              keyboard-visible ring; idle text bumped to neutral-400 (~5.5:1). */}
          <button
            type="button"
            onClick={() => onRemove(v)}
            className="text-neutral-400 hover:text-neutral-100 transition-colors
                       rounded focus:outline-none focus-visible:ring-1
                       focus-visible:ring-accent"
            aria-label={`Remove ${v}`}
          >
            <XIcon className="w-2.5 h-2.5" />
          </button>
        </span>
      ))}
      <input
        type="text"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={onKeyDown}
        onBlur={commit}
        placeholder={values.length === 0 ? placeholder : ""}
        className="flex-1 min-w-[8ch] bg-transparent text-[10px] text-neutral-200
                   placeholder:text-neutral-600 focus:outline-none"
      />
    </div>
  );
}
