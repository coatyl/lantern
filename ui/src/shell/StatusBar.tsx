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
                 bg-surface-1 border-t border-neutral-800/80 shrink-0 select-none"
    >
      {/* Left: document stats. Audit P2 #21: neutral-600 (~3:1) → neutral-400
          (~5.5:1) so stat unit labels clear WCAG AA on surface-1. */}
      {activeInfo ? (
        <span className="flex items-center gap-2 text-[11px] text-neutral-400">
          <span className="tabular-nums">
            {t("statusBar.bookmarks", {
              n: activeInfo.stats.bookmark_count.toLocaleString(),
            })}
          </span>
          <Sep />
          <span className="tabular-nums">
            {t("statusBar.folders", {
              n: activeInfo.stats.folder_count.toLocaleString(),
            })}
          </span>
          <Sep />
          <span className="tabular-nums">
            {t("statusBar.separators", {
              n: activeInfo.stats.separator_count.toLocaleString(),
            })}
          </span>

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
        <span />
      )}

      {/* Right: OFFLINE badge. Audit P2 #21: badge text neutral-600 (~3:1) →
          neutral-300 (~9:1) so the always-visible status reads as AA-strong. */}
      <span
        role="status"
        aria-live="polite"
        className="flex items-center gap-1 px-1.5 py-0.5 rounded
                   text-[10px] font-semibold tracking-wider uppercase
                   bg-neutral-800/70 text-neutral-300"
        title="Lantern never makes network requests unless you explicitly enable the dead-link checker."
      >
        {/* Plug / no-wifi dot */}
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
  return <span className="text-neutral-600" aria-hidden>·</span>;
}
