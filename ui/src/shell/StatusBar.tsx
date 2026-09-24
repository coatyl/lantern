/**
 * Status bar: stats, always-visible OFFLINE indicator (NFR-PR-1), dirty flag.
 *
 * The "modified" label pulses gently via the animate-pulse-dot keyframe so
 * it catches the eye without being distracting.
 */

import { useDocuments } from "../state/documents";
import { useT } from "../i18n/I18nProvider";

export default function StatusBar() {
  const t = useT();
  const { tabs, activeTab } = useDocuments();
  const activeInfo = tabs.find((tab) => tab.id === activeTab) ?? null;

  return (
    <footer
      className="h-6 flex items-center justify-between px-3
                 bg-surface-1 border-t border-[color:var(--border)]/70 shrink-0 select-none"
    >
      {/* Left: document stats (zero counts are left out) */}
      {activeInfo ? (
        <span className="flex items-center gap-2 text-[11px] text-ink-muted">
          {(
            [
              ["statusBar.bookmarks", activeInfo.stats.bookmark_count],
              ["statusBar.folders", activeInfo.stats.folder_count],
              ["statusBar.separators", activeInfo.stats.separator_count],
            ] as const
          )
            .filter(([, n], i) => i === 0 || n > 0)
            .map(([key, n], i) => (
              <span key={key} className="flex items-center gap-2">
                {i > 0 && <Sep />}
                <span className="tabular-nums">{t(key, { n })}</span>
              </span>
            ))}

          {activeInfo.dirty && (
            <>
              <Sep />
              <span className="flex items-center gap-1 text-accent animate-pulse-dot">
                <svg viewBox="0 0 6 6" className="w-1.5 h-1.5 shrink-0" fill="currentColor" aria-hidden>
                  <circle cx="3" cy="3" r="3" />
                </svg>
                {t("statusBar.modified")}
              </span>
            </>
          )}
        </span>
      ) : (
        <span className="text-[11px] text-neutral-400">{t("library.title")}</span>
      )}

      {/* Right: OFFLINE badge */}
      <span
        role="status"
        aria-live="polite"
        className="flex items-center gap-1 px-1.5 py-0.5 rounded
                   text-[10px] font-semibold tracking-wider uppercase
                   bg-surface-3 text-ink-muted"
        title="Lantern never makes network requests unless you explicitly enable the dead-link checker."
      >
        <span className="w-1.5 h-1.5 rounded-full bg-accent lantern-glow shrink-0" aria-hidden />
        <svg viewBox="0 0 8 8" className="w-2 h-2 shrink-0" fill="none" aria-hidden>
          <circle cx="4" cy="4" r="3" stroke="currentColor" strokeWidth="1.2" />
          <line x1="2" y1="2" x2="6" y2="6" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
        </svg>
        {t("statusBar.offline")}
      </span>
    </footer>
  );
}

// ---------------------------------------------------------------------------
// Tiny helpers
// ---------------------------------------------------------------------------

function Sep() {
  // audit P2 #22: separator dot bumped from neutral-800 (invisible against
  // surface-1) to neutral-600 so it actually reads as a divider glyph.
  return <span className="text-ink-faint" aria-hidden>·</span>;
}
