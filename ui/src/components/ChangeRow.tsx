/**
 * A single change-set row: checkbox + field label + destructive pill +
 * before/after diff lines + rationale.
 */

import type { ChangeEntry } from "../ipc/types";
import { DiffLine } from "./DiffLine";

export interface ChangeRowProps {
  change: ChangeEntry;
  approved: boolean;
  onToggle: () => void;
}

export function ChangeRow({ change, approved, onToggle }: ChangeRowProps) {
  return (
    <label
      className="flex items-start gap-2 px-3 py-2 cursor-pointer
                 hover:bg-surface-2 transition-colors"
    >
      <input
        type="checkbox"
        checked={approved}
        onChange={onToggle}
        className="mt-0.5 accent-amber-400 shrink-0"
      />
      <div className="flex-1 min-w-0 text-xs">
        <div className="flex items-center gap-1.5 mb-0.5">
          <span className="text-neutral-500 uppercase tracking-wider text-[10px]">
            {change.field}
          </span>
          {change.destructive && (
            <span className="px-1 rounded bg-danger/20 text-danger text-[10px]">
              destructive
            </span>
          )}
        </div>
        <DiffLine
          spans={change.before_spans}
          fallback={change.before}
          side="before"
        />
        <DiffLine
          spans={change.after_spans}
          fallback={change.after}
          side="after"
        />
        <div className="text-neutral-600 mt-0.5 text-[10px]">{change.rationale}</div>
      </div>
    </label>
  );
}
