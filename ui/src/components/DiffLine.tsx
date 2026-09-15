/**
 * Char-level diff line (before / after).
 *
 * Renders one `<span>` per `DiffSpan` so removed characters get a
 * strikethrough red highlight and added characters get a green highlight.
 * Falls back to a plain string (line-through on the before side) when the
 * backend couldn't compute spans, e.g. the input exceeded MAX_DIFF_INPUT
 * in lantern-core::sanitize::diff.
 */

import type { DiffSpan } from "../ipc/types";

export interface DiffLineProps {
  spans: DiffSpan[] | undefined;
  fallback: string;
  side: "before" | "after";
}

export function DiffLine({ spans, fallback, side }: DiffLineProps) {
  const baseClass =
    side === "before"
      ? "text-neutral-500 truncate text-[11px]"
      : "text-neutral-200 truncate text-[11px]";

  if (!fallback) {
    const placeholderColor =
      side === "before" ? "text-neutral-700" : "text-neutral-600";
    return (
      <div className={baseClass}>
        <em className={`not-italic ${placeholderColor}`}>empty</em>
      </div>
    );
  }

  if (!spans || spans.length === 0) {
    return (
      <div className={side === "before" ? `${baseClass} line-through` : baseClass}>
        {fallback}
      </div>
    );
  }

  return (
    <div className={baseClass}>
      {spans.map((span, i) => {
        if (span.tag === "equal") {
          return <span key={i}>{span.text}</span>;
        }
        if (span.tag === "removed") {
          return (
            <span
              key={i}
              className="bg-diff-removed/20 text-diff-removed line-through rounded-sm px-[1px]"
            >
              {span.text}
            </span>
          );
        }
        return (
          <span
            key={i}
            className="bg-diff-added/20 text-diff-added rounded-sm px-[1px]"
          >
            {span.text}
          </span>
        );
      })}
    </div>
  );
}
