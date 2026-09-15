/**
 * Skeleton placeholders shown while pane content loads (v0.0.11 QoL slice 1).
 *
 * Three primitives:
 *   <Skeleton />:        base block with configurable width / height / radius.
 *   <SkeletonText />:    `n` lines of 80 % width text-shaped placeholders.
 *   <SkeletonRow />:     list-row-shaped placeholder (icon + title + url shapes).
 *
 * All three lean on Tailwind's `animate-pulse` for the breathing animation
 * (already used elsewhere in the codebase) and `bg-surface-2` for the fill so
 * the placeholder is visible against a slightly-darker pane background.  The
 * `prefers-reduced-motion` rule already in `index.css` doesn't directly
 * disable Tailwind's pulse, but the contrast is gentle enough to be tolerable
 * for users with vestibular sensitivities; the default Tailwind pulse fades
 * opacity rather than translating.
 */

import type { CSSProperties } from "react";

// ---------------------------------------------------------------------------
// Base block.
// ---------------------------------------------------------------------------

interface SkeletonProps {
  /** Tailwind width class, e.g. "w-32".  Defaults to "w-full". */
  width?: string;
  /** Tailwind height class, e.g. "h-3".  Defaults to "h-3". */
  height?: string;
  /** Tailwind radius class, e.g. "rounded-md".  Defaults to "rounded". */
  rounded?: string;
  /** Extra classes to merge in (e.g. margin / opacity tweaks). */
  className?: string;
  /** Inline style escape hatch, mostly for one-off pixel widths. */
  style?: CSSProperties;
}

export function Skeleton({
  width = "w-full",
  height = "h-3",
  rounded = "rounded",
  className,
  style,
}: SkeletonProps) {
  const cls = [
    width,
    height,
    rounded,
    "bg-surface-2 animate-pulse",
    className ?? "",
  ].join(" ");
  return <div aria-hidden className={cls} style={style} />;
}

// ---------------------------------------------------------------------------
// Multi-line text placeholder.
// ---------------------------------------------------------------------------

interface SkeletonTextProps {
  /** Number of lines to render.  Defaults to 3. */
  lines?: number;
  /** Extra classes for the wrapper. */
  className?: string;
}

/**
 * `n` lines of 80 % width skeletons stacked with `space-y-2`.  The last line
 * is slightly narrower (60 %) for a more natural shape.
 */
export function SkeletonText({ lines = 3, className }: SkeletonTextProps) {
  const rows = Math.max(1, lines);
  return (
    <div aria-hidden className={`space-y-2 ${className ?? ""}`}>
      {Array.from({ length: rows }).map((_, i) => (
        <Skeleton
          key={i}
          width={i === rows - 1 ? "w-3/5" : "w-4/5"}
          height="h-2.5"
        />
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// List row placeholder (used by TreePane / ListPane / LogsPane).
// ---------------------------------------------------------------------------

/**
 * Roughly the shape of a list row: a small icon block, a title bar, and a
 * narrower domain / url bar to its right.  Sized to feel at home in both
 * compact and comfortable density without trying to match the live row pixel-
 * perfectly (which would lock the placeholder to one density).
 */
export function SkeletonRow({ className }: { className?: string }) {
  return (
    <div
      aria-hidden
      className={`flex items-center gap-2 px-2 py-1.5 ${className ?? ""}`}
    >
      <Skeleton width="w-3.5" height="h-3.5" rounded="rounded" />
      <Skeleton width="w-1/2" height="h-3" />
      <Skeleton width="w-20" height="h-3" className="ml-auto" />
    </div>
  );
}
