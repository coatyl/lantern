/**
 * Inline config editors for configurable treatments: a parameter list for
 * `url.qp.custom`, a pattern + replacement pair for the regex treatments.
 * Built-in (read-only) rule sets get a static rendering instead.
 */

import { useMemo, useState } from "react";
import { XIcon } from "../Icons";
import type { ConfigValue } from "./treatments";

interface ConfigEditorProps {
  readonly: boolean;
  value: ConfigValue;
  onChange: (next: ConfigValue) => void;
}

const INPUT_CLASS =
  "flex-1 min-w-0 bg-surface-3 border rounded px-2 py-1 text-[11px] font-mono " +
  "text-neutral-200 placeholder-neutral-600 focus:outline-none focus:ring-1";

const CHIP_CLASS =
  "text-[10px] font-mono text-neutral-300 bg-surface-3 border border-neutral-700 rounded";

export function TreatmentConfigEditor({ id, ...props }: ConfigEditorProps & { id: string }) {
  if (id === "url.qp.custom") return <ParamsListEditor {...props} />;
  if (id === "title.regex" || id === "folder.regex") return <RegexConfigEditor {...props} />;
  return null;
}

function ParamsListEditor({ readonly, value, onChange }: ConfigEditorProps) {
  const params: string[] = useMemo(() => {
    const v = value?.params;
    return Array.isArray(v) ? v.filter((s): s is string => typeof s === "string") : [];
  }, [value]);

  const [draft, setDraft] = useState("");

  const addDraft = () => {
    const trimmed = draft.trim();
    if (!trimmed) return;
    if (!params.includes(trimmed)) onChange({ params: [...params, trimmed] });
    setDraft("");
  };

  if (readonly) {
    return (
      <div className="px-7 pb-2 -mt-0.5">
        {params.length === 0 ? (
          <p className="text-[10px] text-neutral-600 italic">No parameters configured.</p>
        ) : (
          <div className="flex flex-wrap gap-1">
            {params.map((p) => (
              <span key={p} className={`${CHIP_CLASS} px-1.5 py-0.5`}>
                {p}
              </span>
            ))}
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="px-7 pb-2 -mt-0.5 space-y-1">
      <div className="flex flex-wrap gap-1">
        {params.map((p) => (
          <span key={p} className={`${CHIP_CLASS} pl-1.5 pr-0.5 py-0.5 flex items-center gap-1`}>
            {p}
            <button
              type="button"
              onClick={() => onChange({ params: params.filter((q) => q !== p) })}
              className="text-neutral-600 hover:text-danger focus:outline-none rounded"
              aria-label={`Remove ${p}`}
            >
              <XIcon className="w-2.5 h-2.5" />
            </button>
          </span>
        ))}
      </div>
      <div className="flex items-center gap-1">
        <input
          type="text"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === ",") {
              e.preventDefault();
              addDraft();
            }
          }}
          placeholder="param_name…"
          className={`${INPUT_CLASS} border-neutral-700 focus:ring-accent`}
        />
        <button
          type="button"
          onClick={addDraft}
          disabled={!draft.trim()}
          className="px-2 py-1 rounded text-[11px] text-neutral-400
                     hover:text-neutral-200 hover:bg-surface-3
                     disabled:opacity-40 disabled:hover:bg-transparent
                     focus:outline-none"
        >
          Add
        </button>
      </div>
    </div>
  );
}

function RegexConfigEditor({ readonly, value, onChange }: ConfigEditorProps) {
  const obj = value ?? {};
  const pattern = typeof obj.pattern === "string" ? obj.pattern : "";
  const replacement = typeof obj.replacement === "string" ? obj.replacement : "";

  // Validate client-side so mistakes show inline rather than at save time.
  // An empty pattern is accepted (a no-op).
  const patternError = useMemo(() => {
    if (!pattern) return null;
    try {
      new RegExp(pattern);
      return null;
    } catch (e) {
      return (e as Error).message;
    }
  }, [pattern]);

  const update = (key: "pattern" | "replacement", v: string) => onChange({ ...obj, [key]: v });

  if (readonly) {
    const empty = <em className="text-neutral-600 not-italic">(empty)</em>;
    return (
      <div className="px-7 pb-2 -mt-0.5 grid grid-cols-[auto_1fr] gap-x-2 gap-y-0.5">
        <span className="text-[10px] text-neutral-600 self-center">pattern</span>
        <span className="text-[10px] font-mono text-neutral-300 break-all">{pattern || empty}</span>
        <span className="text-[10px] text-neutral-600 self-center">replace</span>
        <span className="text-[10px] font-mono text-neutral-300 break-all">{replacement || empty}</span>
      </div>
    );
  }

  return (
    <div className="px-7 pb-2 -mt-0.5 space-y-1">
      <div className="flex items-center gap-2">
        <span className="text-[10px] text-neutral-500 w-14 shrink-0">pattern</span>
        <input
          type="text"
          value={pattern}
          onChange={(e) => update("pattern", e.target.value)}
          placeholder="^(.*) - Site name$"
          className={`${INPUT_CLASS} ${patternError
            ? "border-danger/60 focus:ring-danger"
            : "border-neutral-700 focus:ring-accent"}`}
        />
      </div>
      <div className="flex items-center gap-2">
        <span className="text-[10px] text-neutral-500 w-14 shrink-0">replace</span>
        <input
          type="text"
          value={replacement}
          onChange={(e) => update("replacement", e.target.value)}
          placeholder="$1"
          className={`${INPUT_CLASS} border-neutral-700 focus:ring-accent`}
        />
      </div>
      {patternError && (
        <p className="text-[10px] text-danger leading-snug">Invalid regex: {patternError}</p>
      )}
    </div>
  );
}
