import { useState } from "react";
import type { RuleSetSummary } from "../../ipc/types";
import { CopyIcon, LockIcon, PlusIcon, TrashIcon } from "../Icons";
import { treatmentCount } from "./treatments";

/** The inline name prompt shown above the list, if any. */
export type NameInput = { kind: "new" } | { kind: "duplicate"; source: string };

interface RuleSetListProps {
  summaries: RuleSetSummary[];
  loading: boolean;
  selectedName: string | null;
  nameInput: NameInput | null;
  onSelect: (name: string) => void;
  onNameInput: (next: NameInput | null) => void;
  onConfirmName: (name: string) => void;
  onRequestDelete: (name: string) => void;
}

/** Left panel of the rule-set editor: the sets, with new/duplicate/delete. */
export function RuleSetList({
  summaries,
  loading,
  selectedName,
  nameInput,
  onSelect,
  onNameInput,
  onConfirmName,
  onRequestDelete,
}: RuleSetListProps) {
  return (
    <div className="w-60 shrink-0 border-r border-neutral-800 flex flex-col bg-surface-0">
      <div className="px-3 py-3 border-b border-neutral-800 flex items-center justify-between shrink-0">
        <h3 className="text-xs font-semibold text-neutral-400 uppercase tracking-wider">Rule Sets</h3>
        <button
          onClick={() => onNameInput({ kind: "new" })}
          title="New rule set"
          className="text-neutral-500 hover:text-neutral-200
                     transition-colors focus:outline-none
                     focus-visible:ring-1 focus-visible:ring-accent"
        >
          <PlusIcon className="w-3.5 h-3.5" />
        </button>
      </div>

      {nameInput && (
        <InlineNameInput
          // Remount when the prompt changes so the draft starts empty.
          key={nameInput.kind === "new" ? "new" : `dup:${nameInput.source}`}
          placeholder={nameInput.kind === "new" ? "Rule set name…" : `Copy of ${nameInput.source}…`}
          onConfirm={onConfirmName}
          onCancel={() => onNameInput(null)}
        />
      )}

      <div className="flex-1 overflow-y-auto py-1 px-1 space-y-0.5">
        {loading && summaries.length === 0 ? (
          <p className="px-3 py-3 text-xs text-neutral-600">Loading…</p>
        ) : (
          summaries.map((s) => (
            <div key={s.name} className="group relative">
              <button
                onClick={() => onSelect(s.name)}
                className={`w-full text-left px-3 py-2 rounded transition-colors
                            focus:outline-none focus-visible:ring-1 focus-visible:ring-accent
                            ${selectedName === s.name
                              ? "bg-surface-3 text-neutral-100"
                              : "text-neutral-400 hover:bg-surface-2 hover:text-neutral-200"}`}
              >
                <div className="flex items-center gap-1.5 min-w-0">
                  {s.is_builtin && <LockIcon className="w-2.5 h-2.5 text-neutral-600 shrink-0" />}
                  <span className="text-xs font-medium truncate">{s.name}</span>
                </div>
                <p className="text-[10px] text-neutral-600 mt-0.5 truncate">
                  {treatmentCount(s.treatment_count)}
                </p>
              </button>

              {/* Row actions: shown on hover, and while one has keyboard focus. */}
              <div
                className="absolute right-1 top-1/2 -translate-y-1/2 flex items-center gap-0.5
                           opacity-0 group-hover:opacity-100 focus-within:opacity-100"
              >
                <button
                  onClick={() => onNameInput({ kind: "duplicate", source: s.name })}
                  title="Duplicate"
                  className="p-1 rounded text-neutral-600
                             hover:text-neutral-300 hover:bg-surface-3
                             focus:outline-none focus-visible:ring-1 focus-visible:ring-accent"
                >
                  <CopyIcon className="w-2.5 h-2.5" />
                </button>
                {!s.is_builtin && (
                  <button
                    onClick={() => onRequestDelete(s.name)}
                    title="Delete"
                    className="p-1 rounded text-neutral-600
                               hover:text-danger hover:bg-surface-3
                               focus:outline-none focus-visible:ring-1 focus-visible:ring-accent"
                  >
                    <TrashIcon className="w-2.5 h-2.5" />
                  </button>
                )}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

function InlineNameInput({
  placeholder,
  onConfirm,
  onCancel,
}: {
  placeholder: string;
  onConfirm: (name: string) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState("");
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        const name = value.trim();
        if (name) onConfirm(name);
      }}
      className="flex items-center gap-1 px-2 py-1.5"
    >
      <input
        autoFocus
        type="text"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder={placeholder}
        className="flex-1 min-w-0 bg-surface-3 border border-accent/40
                   rounded px-2 py-1 text-xs text-neutral-200
                   placeholder-neutral-600 focus:outline-none
                   focus:ring-1 focus:ring-accent"
      />
      <button
        type="submit"
        disabled={!value.trim()}
        className="px-2 py-1 rounded text-xs font-medium
                   bg-accent disabled:opacity-40 text-on-accent
                   focus:outline-none"
      >
        OK
      </button>
      <button
        type="button"
        onClick={onCancel}
        aria-label="Cancel"
        className="px-1.5 py-1 text-neutral-500 hover:text-neutral-300
                   focus:outline-none text-xs"
      >
        ✕
      </button>
    </form>
  );
}
