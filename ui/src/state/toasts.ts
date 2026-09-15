/**
 * Zustand store for transient toast notifications (v0.0.11 QoL slice 1).
 *
 * Toasts are append-only short-lived messages anchored at the bottom-right of
 * the app.  The store holds the active list, a tiny enqueue helper that
 * generates a unique id + (optionally) schedules an auto-dismiss timer, and
 * an explicit `dismiss(id)` action used by the inline × button or by the
 * timer callback.
 *
 * Auto-dismiss policy (matches the v0.0.11 PRD):
 *   info     →  4s
 *   success  →  6s
 *   error    →  never (must be dismissed)
 *
 * Errors stay sticky on purpose: the user must acknowledge them, and screen
 * readers (via `aria-live="assertive"` on the error region; see Toast.tsx)
 * announce them as soon as they land.
 */

import { create } from "zustand";

export type ToastKind = "info" | "success" | "error";

export interface ToastAction {
  label: string;
  onClick: () => void;
}

export interface Toast {
  id: string;
  kind: ToastKind;
  message: string;
  action?: ToastAction;
}

const AUTO_DISMISS_MS: Record<ToastKind, number | null> = {
  info: 4_000,
  success: 6_000,
  error: null,
};

let nextSeq = 0;
function makeId(): string {
  nextSeq += 1;
  return `toast-${Date.now().toString(36)}-${nextSeq}`;
}

interface ToastState {
  toasts: Toast[];
  /** Enqueue a toast.  Returns the new id so callers can dismiss it early. */
  push: (msg: string, kind?: ToastKind, action?: ToastAction) => string;
  /** Remove a toast by id.  No-op if the id isn't present. */
  dismiss: (id: string) => void;
  /** Drop every active toast (used by tests + edge cases like locale change). */
  clear: () => void;
}

export const useToastStore = create<ToastState>((set, get) => ({
  toasts: [],

  push: (msg, kind = "info", action) => {
    const id = makeId();
    const toast: Toast = { id, kind, message: msg, action };
    set((s) => ({ toasts: [...s.toasts, toast] }));

    const ttl = AUTO_DISMISS_MS[kind];
    if (ttl !== null && typeof window !== "undefined") {
      window.setTimeout(() => get().dismiss(id), ttl);
    }
    return id;
  },

  dismiss: (id) => {
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
  },

  clear: () => set({ toasts: [] }),
}));
