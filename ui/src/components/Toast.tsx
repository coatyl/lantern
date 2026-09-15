/**
 * Tiny custom toast / notification surface (v0.0.11 QoL slice 1).
 *
 * No third-party dependency: the toast list lives in a Zustand store
 * (`state/toasts.ts`) and is read here so the renderer is purely
 * presentational.  Auto-dismissal happens in the store; this component just
 * renders whatever's currently in the list.
 *
 * Layout
 * ------
 * Fixed at the bottom-right of the viewport, stacked top-down with `gap-2`.
 * Sits at z-index 70, above modal overlays (z-50) but below the global
 * focus ring layer that browsers paint above everything.
 *
 * A11y
 * ----
 * Errors land in an `aria-live="assertive"` region so screen readers
 * announce them immediately; info / success toasts share a polite region.
 * Each toast has an inline × button for manual dismiss.  Clicks outside the
 * toast do *not* dismiss it; the only ways out are timeout, button, or an
 * explicit `dismiss()` call.
 *
 * The default export is `<Toaster />`, which is what `App.tsx` mounts.
 */

import type { ReactNode } from "react";
import { useToast } from "../hooks/useToast";
import { useT } from "../i18n/I18nProvider";
import type { Toast as ToastModel, ToastKind } from "../state/toasts";
import { XIcon } from "./Icons";

export type { Toast, ToastKind } from "../state/toasts";

// ---------------------------------------------------------------------------
// Tone classes: keep colour decisions in one place.
// ---------------------------------------------------------------------------

interface ToneClasses {
  container: string;
  icon: string;
}

const TONE: Record<ToastKind, ToneClasses> = {
  info: {
    container: "bg-surface-2 border-neutral-700 text-neutral-200",
    icon: "text-neutral-400",
  },
  success: {
    container: "bg-surface-2 border-diff-added/40 text-neutral-100",
    icon: "text-diff-added",
  },
  error: {
    container: "bg-surface-2 border-danger/60 text-neutral-100",
    icon: "text-danger",
  },
};

// ---------------------------------------------------------------------------
// Per-tone glyph: small, purely decorative SVGs (`aria-hidden`).
// ---------------------------------------------------------------------------

function ToneIcon({ kind, className }: { kind: ToastKind; className?: string }): ReactNode {
  const cls = `w-4 h-4 shrink-0 ${className ?? ""}`;
  if (kind === "success") {
    return (
      <svg viewBox="0 0 16 16" className={cls} aria-hidden fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M3 8.5l3.5 3.5L13 5" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    );
  }
  if (kind === "error") {
    return (
      <svg viewBox="0 0 16 16" className={cls} aria-hidden fill="none" stroke="currentColor" strokeWidth="2">
        <circle cx="8" cy="8" r="6.5" />
        <path d="M8 4.5v4M8 11h.01" strokeLinecap="round" />
      </svg>
    );
  }
  return (
    <svg viewBox="0 0 16 16" className={cls} aria-hidden fill="none" stroke="currentColor" strokeWidth="2">
      <circle cx="8" cy="8" r="6.5" />
      <path d="M8 7.5v4M8 5h.01" strokeLinecap="round" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Single toast row.
// ---------------------------------------------------------------------------

function ToastRow({
  toast,
  onDismiss,
  dismissLabel,
}: {
  toast: ToastModel;
  onDismiss: (id: string) => void;
  dismissLabel: string;
}) {
  const tone = TONE[toast.kind];
  return (
    <div
      role={toast.kind === "error" ? "alert" : "status"}
      data-toast
      data-kind={toast.kind}
      className={`relative w-72 max-w-[90vw] rounded-md border shadow-lg
                  px-3 py-2 pr-8 text-xs flex items-start gap-2
                  animate-toast-in ${tone.container}`}
    >
      <ToneIcon kind={toast.kind} className={tone.icon} />
      <div className="flex-1 min-w-0">
        <p className="leading-snug break-words">{toast.message}</p>
        {toast.action && (
          <button
            type="button"
            onClick={() => {
              toast.action?.onClick();
              onDismiss(toast.id);
            }}
            className="mt-1 text-[11px] font-medium text-accent hover:text-accent-hover
                       focus:outline-none focus-visible:ring-1 focus-visible:ring-accent
                       rounded px-1 -mx-1"
          >
            {toast.action.label}
          </button>
        )}
      </div>
      <button
        type="button"
        onClick={() => onDismiss(toast.id)}
        aria-label={dismissLabel}
        className="absolute top-1 right-1 inline-flex items-center justify-center
                   w-6 h-6 rounded text-neutral-500 hover:text-neutral-200
                   focus:outline-none focus-visible:ring-1 focus-visible:ring-accent"
      >
        <XIcon className="w-3 h-3" />
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Public renderer: mounted once at the App root.
// ---------------------------------------------------------------------------

export default function Toaster() {
  const { toasts, dismiss } = useToast();
  const t = useT();
  const dismissLabel = t("toast.dismiss");

  // Errors live in an assertive region (announced immediately); info / success
  // share a polite region (queued).
  const errorToasts = toasts.filter((toast) => toast.kind === "error");
  const otherToasts = toasts.filter((toast) => toast.kind !== "error");

  return (
    <>
      <div
        aria-live="polite"
        aria-atomic="false"
        className="fixed bottom-3 right-3 z-[70] flex flex-col items-end gap-2
                   pointer-events-none"
      >
        {otherToasts.map((toast) => (
          <div key={toast.id} className="pointer-events-auto">
            <ToastRow toast={toast} onDismiss={dismiss} dismissLabel={dismissLabel} />
          </div>
        ))}
      </div>
      <div
        aria-live="assertive"
        aria-atomic="false"
        className="fixed bottom-3 right-3 z-[70] flex flex-col items-end gap-2
                   pointer-events-none"
      >
        {errorToasts.map((toast, idx) => (
          <div
            key={toast.id}
            className="pointer-events-auto"
            // Stack errors above the polite region by translating up one slot
            // per non-error toast already on screen.  Both regions render at
            // the same anchor, so without this offset they'd overlap.
            style={{ marginBottom: idx === 0 ? otherToasts.length * 4 : 0 }}
          >
            <ToastRow toast={toast} onDismiss={dismiss} dismissLabel={dismissLabel} />
          </div>
        ))}
      </div>
    </>
  );
}
