/**
 * Generic small popover menu anchored to a coordinate.
 *
 * Rendered as a fixed-position floating element so it can be opened from any
 * surface without worrying about parent layout / clipping.  Position is
 * clipped to the viewport edge so a right-click near the right or bottom
 * border doesn't render the menu off-screen; we measure once on mount and
 * shift the menu inwards if it overflows.
 *
 * Keyboard contract (matches WAI-ARIA "menu" role):
 *  - On open, focus moves to the first non-disabled item.
 *  - ArrowDown / ArrowUp move focus through enabled items, wrapping at the
 *    edges; Home / End jump to the first / last enabled item.
 *  - Enter or Space activates the focused item (which also closes the menu).
 *  - Escape closes the menu and calls `onClose`.
 *  - A click outside the menu closes it.
 *
 * Items can carry a `separator` flag to render as a non-interactive divider;
 * separators are skipped by the keyboard nav and aren't counted as menuitems.
 */

import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKE,
} from "react";

export interface ContextMenuItem {
  /** Stable id used as the React key. */
  id: string;
  /** Visible label; ignored when `separator` is true. */
  label?: string;
  /** When true, render a non-interactive divider; `label` / `onSelect` ignored. */
  separator?: boolean;
  /** Disabled items render greyed-out and are skipped by keyboard nav. */
  disabled?: boolean;
  /** Invoked when the item is clicked or activated via Enter/Space. */
  onSelect?: () => void;
}

export interface ContextMenuProps {
  /** Anchor point in viewport coordinates (typically clientX / clientY). */
  x: number;
  y: number;
  /** The menu items to render; interleave separator entries as needed. */
  items: ContextMenuItem[];
  /** Called when the menu wants to close (outside click, Escape, item activation). */
  onClose: () => void;
  /** Optional accessible name for the menu. */
  ariaLabel?: string;
}

export function ContextMenu({ x, y, items, onClose, ariaLabel }: ContextMenuProps) {
  const menuRef = useRef<HTMLDivElement | null>(null);

  // Position state: start at the requested coordinate, then clamp to the
  // viewport once we know the rendered size.  Hidden until measured to
  // avoid a one-frame flash at the wrong spot.
  const [pos, setPos] = useState<{ x: number; y: number; ready: boolean }>({
    x,
    y,
    ready: false,
  });

  useLayoutEffect(() => {
    const node = menuRef.current;
    if (!node) return;
    const rect = node.getBoundingClientRect();
    const margin = 4;
    let nx = x;
    let ny = y;
    if (nx + rect.width > window.innerWidth - margin) {
      nx = Math.max(margin, window.innerWidth - rect.width - margin);
    }
    if (ny + rect.height > window.innerHeight - margin) {
      ny = Math.max(margin, window.innerHeight - rect.height - margin);
    }
    if (nx < margin) nx = margin;
    if (ny < margin) ny = margin;
    setPos({ x: nx, y: ny, ready: true });
  }, [x, y]);

  // ── Outside click + Escape close ─────────────────────────────────────
  useEffect(() => {
    const onPointerDown = (e: MouseEvent) => {
      if (!menuRef.current?.contains(e.target as Node)) {
        onClose();
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("mousedown", onPointerDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onPointerDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  // ── Initial focus ────────────────────────────────────────────────────
  useEffect(() => {
    const menu = menuRef.current;
    if (!menu) return;
    const first = menu.querySelector<HTMLButtonElement>(
      'button[role="menuitem"]:not([disabled])',
    );
    first?.focus();
  }, []);

  // ── Local keyboard nav ───────────────────────────────────────────────
  const handleKeyDown = (e: ReactKE<HTMLDivElement>) => {
    const menu = menuRef.current;
    if (!menu) return;
    const enabled = Array.from(
      menu.querySelectorAll<HTMLButtonElement>(
        'button[role="menuitem"]:not([disabled])',
      ),
    );
    if (enabled.length === 0) return;
    const idx = enabled.indexOf(document.activeElement as HTMLButtonElement);

    if (e.key === "ArrowDown") {
      e.preventDefault();
      const next = idx < enabled.length - 1 ? idx + 1 : 0;
      enabled[next].focus();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      const prev = idx > 0 ? idx - 1 : enabled.length - 1;
      enabled[prev].focus();
    } else if (e.key === "Home") {
      e.preventDefault();
      enabled[0].focus();
    } else if (e.key === "End") {
      e.preventDefault();
      enabled[enabled.length - 1].focus();
    } else if (e.key === "Enter" || e.key === " ") {
      // Default button click handles activation, but explicitly activating
      // here ensures the keyboard path is identical for screen-reader virtual
      // buffers that don't fire button-click on Space.
      const focused = document.activeElement as HTMLButtonElement | null;
      if (focused && enabled.includes(focused)) {
        e.preventDefault();
        focused.click();
      }
    }
  };

  return (
    <div
      ref={menuRef}
      role="menu"
      aria-label={ariaLabel}
      onKeyDown={handleKeyDown}
      style={{
        position: "fixed",
        left: pos.x,
        top: pos.y,
        visibility: pos.ready ? "visible" : "hidden",
        zIndex: 100,
      }}
      className="min-w-[180px] rounded-md border border-neutral-800
                 bg-surface-1 shadow-2xl py-1 text-xs"
    >
      {items.map((item) => {
        if (item.separator) {
          return (
            <div
              key={item.id}
              role="separator"
              aria-orientation="horizontal"
              className="my-1 border-t border-neutral-800"
            />
          );
        }
        return (
          <button
            key={item.id}
            type="button"
            role="menuitem"
            disabled={item.disabled}
            onClick={() => {
              if (item.disabled) return;
              item.onSelect?.();
              onClose();
            }}
            className="w-full text-left px-3 py-1.5 text-neutral-200
                       hover:bg-surface-2 disabled:opacity-40
                       disabled:hover:bg-transparent transition-colors
                       focus:outline-none focus-visible:bg-surface-2
                       focus:bg-surface-2"
          >
            {item.label}
          </button>
        );
      })}
    </div>
  );
}

export default ContextMenu;
