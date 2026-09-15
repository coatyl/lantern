/**
 * useFocusTrap: keyboard focus management for modal dialogs.
 *
 * Behaviour (matches the WAI-ARIA dialog authoring practice):
 *  - On mount: focus moves to the first focusable element inside the container
 *    (skipping any element marked with `data-autofocus-skip="true"`).
 *  - On Tab / Shift+Tab: focus cycles among the focusable elements inside the
 *    container; never escapes to the underlying page.
 *  - On Escape: invokes the `onClose` callback so the modal can dismiss itself.
 *  - On unmount: restores focus to the element that was focused when the trap
 *    activated, so closing the dialog returns the user where they started.
 *
 * Usage:
 *
 *   const ref = useFocusTrap<HTMLDivElement>(open, onClose);
 *   return open ? <div ref={ref} role="dialog">…</div> : null;
 *
 * The hook is a no-op while `active` is false, so it's safe to call
 * unconditionally in components that conditionally render their dialog.
 */
import { useEffect, useRef } from "react";

const FOCUSABLE_SELECTOR = [
  "a[href]",
  "area[href]",
  "button:not([disabled])",
  "input:not([disabled]):not([type='hidden'])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "iframe",
  "object",
  "embed",
  "[tabindex]:not([tabindex='-1'])",
  "[contenteditable='true']",
].join(",");

function getFocusable(container: HTMLElement): HTMLElement[] {
  const nodes = container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR);
  return Array.from(nodes).filter((el) => {
    if (el.hasAttribute("disabled")) return false;
    if (el.getAttribute("aria-hidden") === "true") return false;
    // Hidden via CSS `display:none` or the `hidden` attribute, so skip.
    // (We avoid checking `offsetParent` here because jsdom and other
    // headless environments report `null` for everything regardless of
    // visibility.  `getComputedStyle` is implemented in jsdom and gives a
    // reliable signal.)
    if (el.hidden) return false;
    if (typeof window !== "undefined") {
      const style = window.getComputedStyle(el);
      if (style.display === "none" || style.visibility === "hidden") return false;
    }
    return true;
  });
}

export function useFocusTrap<T extends HTMLElement = HTMLElement>(
  active: boolean,
  onClose: () => void,
) {
  const ref = useRef<T | null>(null);
  // Stash the latest onClose without re-binding the keydown listener every
  // render; consumers usually pass an inline arrow function.
  const onCloseRef = useRef(onClose);
  useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  useEffect(() => {
    if (!active) return;
    const container = ref.current;
    if (!container) return;

    // Remember where focus came from so we can restore it on close.
    const previouslyFocused =
      typeof document !== "undefined"
        ? (document.activeElement as HTMLElement | null)
        : null;

    // Move focus inside the modal.  Prefer the first focusable element that
    // isn't explicitly opted out via `data-autofocus-skip`; fall back to the
    // container itself (made programmatically focusable) so screen readers
    // anchor inside the dialog.
    const focusables = getFocusable(container);
    const target = focusables.find(
      (el) => el.getAttribute("data-autofocus-skip") !== "true",
    );
    if (target) {
      target.focus();
    } else {
      // Container needs tabindex to receive focus.
      if (!container.hasAttribute("tabindex")) {
        container.setAttribute("tabindex", "-1");
      }
      container.focus();
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        event.preventDefault();
        onCloseRef.current();
        return;
      }
      if (event.key !== "Tab") return;

      const items = getFocusable(container);
      if (items.length === 0) {
        // No focusable descendants: keep focus pinned to the container.
        event.preventDefault();
        container.focus();
        return;
      }

      const first = items[0];
      const last = items[items.length - 1];
      const activeEl = document.activeElement as HTMLElement | null;

      if (event.shiftKey) {
        // Shift+Tab from the first element wraps to the last.
        if (activeEl === first || !container.contains(activeEl)) {
          event.preventDefault();
          last.focus();
        }
      } else {
        // Tab from the last element wraps to the first.
        if (activeEl === last || !container.contains(activeEl)) {
          event.preventDefault();
          first.focus();
        }
      }
    };

    container.addEventListener("keydown", handleKeyDown);
    return () => {
      container.removeEventListener("keydown", handleKeyDown);
      // Restore focus to the trigger element when the trap deactivates.
      if (
        previouslyFocused &&
        typeof previouslyFocused.focus === "function" &&
        document.contains(previouslyFocused)
      ) {
        previouslyFocused.focus();
      }
    };
  }, [active]);

  return ref;
}
