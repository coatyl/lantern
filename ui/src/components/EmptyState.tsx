/**
 * Generic empty-state surface (v0.0.11 QoL slice 1).
 *
 * Vertically-centered icon + bold title + optional small description + an
 * optional accent action button.  The wrapping div carries an
 * `aria-label` so a screen-reader user lands on a meaningfully announced
 * region rather than an unstructured cluster of strings.
 *
 * Usage
 * -----
 * Three callers right now:
 *   - TreePane:      "no folders yet"
 *   - ListPane:      empty folder + no search results
 *   - DeadLinkModal: pre-run + all-green
 *   - LogsPane:      no log entries yet
 *
 * Each caller is expected to import an icon (or use the default dot one) and
 * pass localised `title` / `description` strings.  See `i18n/en.ts` for the
 * shared keys.
 */

import type { ReactNode } from "react";

interface EmptyStateProps {
  /** Optional icon.  Defaults to a generic dot/circle SVG. */
  icon?: ReactNode;
  /** Bold heading. */
  title: string;
  /** Small explainer text.  Optional. */
  description?: string;
  /** Optional accent CTA. */
  action?: { label: string; onClick: () => void };
  /** Optional aria-label.  Defaults to the title. */
  ariaLabel?: string;
  /** Extra classes to merge into the outer wrapper. */
  className?: string;
}

const DEFAULT_ICON = (
  <svg viewBox="0 0 24 24" className="w-7 h-7" aria-hidden fill="none" stroke="currentColor" strokeWidth="1.6">
    <circle cx="12" cy="12" r="9" />
    <circle cx="12" cy="12" r="2.5" />
  </svg>
);

export function EmptyState({
  icon,
  title,
  description,
  action,
  ariaLabel,
  className,
}: EmptyStateProps) {
  return (
    <div
      role="region"
      aria-label={ariaLabel ?? title}
      className={`flex-1 flex flex-col items-center justify-center
                  px-4 py-6 text-center gap-2 select-none ${className ?? ""}`}
    >
      <div className="text-accent/80" aria-hidden>
        {icon ?? DEFAULT_ICON}
      </div>
      <p className="font-display-tight text-sm text-ink">{title}</p>
      {description && (
        <p className="text-[11px] text-ink-muted max-w-xs leading-relaxed">
          {description}
        </p>
      )}
      {action && (
        <button
          type="button"
          onClick={action.onClick}
          className="mt-1 px-3 py-1 rounded text-[11px] font-medium
                     bg-accent text-on-accent hover:bg-accent-hover
                     transition-colors focus:outline-none
                     focus-visible:ring-2 focus-visible:ring-accent"
        >
          {action.label}
        </button>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Pre-baked icons: small line-art SVGs the consumers can drop in for
// recognisable shapes without each one re-deriving the artwork.
// ---------------------------------------------------------------------------

export function EmptyFolderIcon() {
  return (
    <svg viewBox="0 0 24 24" className="w-7 h-7" aria-hidden fill="none" stroke="currentColor" strokeWidth="1.6">
      <path d="M3 7.5A1.5 1.5 0 0 1 4.5 6h4.2L11 8h8.5A1.5 1.5 0 0 1 21 9.5v8A1.5 1.5 0 0 1 19.5 19h-15A1.5 1.5 0 0 1 3 17.5v-10z" />
    </svg>
  );
}

export function EmptySearchIcon() {
  return (
    <svg viewBox="0 0 24 24" className="w-7 h-7" aria-hidden fill="none" stroke="currentColor" strokeWidth="1.6">
      <circle cx="11" cy="11" r="6" />
      <path d="M20 20l-4.5-4.5" strokeLinecap="round" />
    </svg>
  );
}

export function EmptyCheckIcon() {
  return (
    <svg viewBox="0 0 24 24" className="w-7 h-7" aria-hidden fill="none" stroke="currentColor" strokeWidth="1.6">
      <circle cx="12" cy="12" r="9" />
      <path d="M8 12.5l3 3 5-6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function EmptyDocumentIcon() {
  return (
    <svg viewBox="0 0 24 24" className="w-7 h-7" aria-hidden fill="none" stroke="currentColor" strokeWidth="1.6">
      <path d="M7 3h7l4 4v13.5A1.5 1.5 0 0 1 16.5 22h-9A1.5 1.5 0 0 1 6 20.5v-16A1.5 1.5 0 0 1 7.5 3H7z" />
      <path d="M14 3v4h4" />
    </svg>
  );
}
