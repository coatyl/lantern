/**
 * Convenience hook over the toast Zustand store.
 *
 * `toast(msg, kind?, action?)` enqueues a new toast and returns its id;
 * `toasts` is the live array for `<Toaster />` to render; `dismiss(id)`
 * removes a specific toast (used by the inline × button or by callers that
 * want to acknowledge their own toast, e.g. clearing a long-running error
 * after a manual retry succeeds).
 *
 * Components that only need to push toasts can pull just `toast` and stay
 * insulated from the rest of the store.
 */

import { useToastStore, type Toast, type ToastAction, type ToastKind } from "../state/toasts";

export interface UseToastReturn {
  toast: (msg: string, kind?: ToastKind, action?: ToastAction) => string;
  toasts: Toast[];
  dismiss: (id: string) => void;
}

export function useToast(): UseToastReturn {
  const toasts = useToastStore((s) => s.toasts);
  const push = useToastStore((s) => s.push);
  const dismiss = useToastStore((s) => s.dismiss);
  return { toast: push, toasts, dismiss };
}
