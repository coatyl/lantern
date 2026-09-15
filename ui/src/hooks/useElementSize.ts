/**
 * useElementSize: measure a DOM element's content-box size and keep it in
 * sync via `ResizeObserver`.
 *
 * Returns a ref to attach to the element you want to measure plus the latest
 * `{ width, height }` reading.  The first render returns `{ width: 0, height: 0 }`;
 * a layout effect kicks the observer immediately so a measurement lands
 * before paint when the element is already in the tree.
 *
 * Used by ListPane to feed `react-window`'s `FixedSizeList` a concrete pixel
 * height for the row viewport; `react-window` doesn't auto-size to its
 * parent.  Kept tiny on purpose: no debouncing, no breakpoint logic.
 */

import { useCallback, useLayoutEffect, useRef, useState } from "react";

export interface ElementSize {
  width:  number;
  height: number;
}

/**
 * Returns a ref-callback to attach to the element under measurement and the
 * latest observed size.  We intentionally use a ref-callback (not `useRef`)
 * so the observer rebinds when the underlying element is swapped, e.g. when
 * the parent conditionally renders a different wrapper.
 */
export function useElementSize<T extends Element = HTMLElement>(): {
  ref:    (node: T | null) => void;
  width:  number;
  height: number;
} {
  const [size, setSize] = useState<ElementSize>({ width: 0, height: 0 });

  // Held outside React state so cleanup can disconnect without triggering
  // re-renders.
  const observerRef = useRef<ResizeObserver | null>(null);
  const nodeRef     = useRef<T | null>(null);

  // Reusable measurement step: fires from both the initial layout effect
  // and the ResizeObserver callback.
  const measure = useCallback((el: Element) => {
    const rect = el.getBoundingClientRect();
    setSize((prev) => {
      if (prev.width === rect.width && prev.height === rect.height) return prev;
      return { width: rect.width, height: rect.height };
    });
  }, []);

  const ref = useCallback((node: T | null) => {
    // Disconnect the previous observer when the underlying node is swapped.
    if (observerRef.current) {
      observerRef.current.disconnect();
      observerRef.current = null;
    }
    nodeRef.current = node;
    if (!node) return;

    // Synchronous first measurement so the consumer doesn't render a 0-px
    // viewport on mount when the node already has layout.
    measure(node);

    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      measure(entry.target);
    });
    observer.observe(node);
    observerRef.current = observer;
  }, [measure]);

  // If the consumer never swaps the node, the ref-callback runs once at mount
  // and we're done.  This effect only handles the unmount cleanup branch,
  // since the callback already disconnects on swap.
  useLayoutEffect(() => {
    return () => {
      observerRef.current?.disconnect();
      observerRef.current = null;
    };
  }, []);

  return { ref, width: size.width, height: size.height };
}
