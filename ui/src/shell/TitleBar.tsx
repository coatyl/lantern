/**
 * Custom title bar (the main window is frameless).
 *
 * Provides: app name, active-document name, Export button, drag region, and
 * native-style SVG window controls (minimise / maximise / close).
 */

import { useEffect, useRef, useState, type KeyboardEvent as ReactKE } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { save } from "@tauri-apps/plugin-dialog";
import { useDocuments } from "../state/documents";
import { ipc } from "../ipc";
import { useT } from "../i18n/I18nProvider";
import { ChevronDownIcon, GearIcon, HomeIcon, LinkIcon } from "../components/Icons";

const appWindow = getCurrentWindow();

export default function TitleBar({
  onSettings,
  onCompareTabs,
  onCheckDeadLinks,
  onMergeDocuments,
  canCompareTabs = false,
  canCheckDeadLinks = false,
  canMergeDocuments = false,
}: {
  onSettings?: () => void;
  onCompareTabs?: () => void;
  onCheckDeadLinks?: () => void;
  onMergeDocuments?: () => void;
  canCompareTabs?: boolean;
  canCheckDeadLinks?: boolean;
  canMergeDocuments?: boolean;
}) {
  const t = useT();
  const { activeTab, tabs, showLibrary } = useDocuments();
  const [exporting, setExporting] = useState(false);
  const [toolsOpen, setToolsOpen] = useState(false);
  const toolsRef = useRef<HTMLDivElement | null>(null);
  const toolsTriggerRef = useRef<HTMLButtonElement | null>(null);
  const toolsMenuRef = useRef<HTMLDivElement | null>(null);

  const activeInfo = tabs.find((t) => t.id === activeTab) ?? null;

  useEffect(() => {
    if (!toolsOpen) return;
    const handlePointerDown = (e: MouseEvent) => {
      if (!toolsRef.current?.contains(e.target as Node)) {
        setToolsOpen(false);
      }
    };
    window.addEventListener("mousedown", handlePointerDown);
    return () => window.removeEventListener("mousedown", handlePointerDown);
  }, [toolsOpen]);

  // Move focus to the first menu item when the menu opens via keyboard.
  useEffect(() => {
    if (!toolsOpen) return;
    const menu = toolsMenuRef.current;
    if (!menu) return;
    const items = menu.querySelectorAll<HTMLButtonElement>('button[role="menuitem"]');
    items[0]?.focus();
  }, [toolsOpen]);

  const closeToolsMenu = (returnFocus = true) => {
    setToolsOpen(false);
    if (returnFocus) {
      // Defer focus restoration until after the menu unmounts.
      requestAnimationFrame(() => toolsTriggerRef.current?.focus());
    }
  };

  const handleTriggerKeyDown = (e: ReactKE<HTMLButtonElement>) => {
    if (e.key === " " || e.key === "Enter" || e.key === "ArrowDown") {
      e.preventDefault();
      setToolsOpen(true);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setToolsOpen(true);
      // Focus last item once menu mounts.
      requestAnimationFrame(() => {
        const items =
          toolsMenuRef.current?.querySelectorAll<HTMLButtonElement>(
            'button[role="menuitem"]',
          );
        items?.[items.length - 1]?.focus();
      });
    } else if (e.key === "Escape" && toolsOpen) {
      e.preventDefault();
      closeToolsMenu();
    }
  };

  const handleMenuKeyDown = (e: ReactKE<HTMLDivElement>) => {
    const menu = toolsMenuRef.current;
    if (!menu) return;
    const items = Array.from(
      menu.querySelectorAll<HTMLButtonElement>('button[role="menuitem"]'),
    );
    if (items.length === 0) return;
    const currentIdx = items.indexOf(document.activeElement as HTMLButtonElement);

    if (e.key === "ArrowDown") {
      e.preventDefault();
      const next = currentIdx < items.length - 1 ? currentIdx + 1 : 0;
      items[next].focus();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      const prev = currentIdx > 0 ? currentIdx - 1 : items.length - 1;
      items[prev].focus();
    } else if (e.key === "Home") {
      e.preventDefault();
      items[0].focus();
    } else if (e.key === "End") {
      e.preventDefault();
      items[items.length - 1].focus();
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      closeToolsMenu();
    } else if (e.key === "Tab") {
      // Tabbing out closes the menu without restoring focus to the trigger
      // (the user is moving on intentionally).
      closeToolsMenu(false);
    }
  };

  const handleExport = async () => {
    if (!activeTab) return;
    setExporting(true);
    try {
      const path = await save({
        filters: [{ name: "HTML bookmark file", extensions: ["html", "htm"] }],
        defaultPath: "bookmarks.html",
      });
      if (typeof path === "string") {
        await ipc.export(activeTab, { kind: "whole_document" }, path);
      }
    } catch {
      // user cancelled or export failed: ignore
    } finally {
      setExporting(false);
    }
  };

  return (
    <header
      data-tauri-drag-region
      className="h-8 flex items-center justify-between bg-surface-1
                 border-b border-neutral-800/80 shrink-0 select-none"
    >
      {/* ── Left: brand name ─────────────────────────────────────────────── */}
      <div
        data-tauri-drag-region
        className="flex items-center gap-2 pl-3 w-32 shrink-0"
      >
        {/* Micro lantern icon */}
        <svg
          viewBox="0 0 14 18"
          className="w-3 h-3.5 shrink-0 text-accent"
          fill="currentColor"
          aria-hidden
        >
          <rect x="3" y="3.5" width="8" height="11" rx="1.5" opacity="0.25" />
          <rect x="3" y="3.5" width="8" height="11" rx="1.5"
                fill="none" stroke="currentColor" strokeWidth="1.2" />
          <line x1="3" y1="9" x2="11" y2="9"
                stroke="currentColor" strokeWidth="0.8" opacity="0.5" />
          <line x1="7" y1="3.5" x2="7" y2="14.5"
                stroke="currentColor" strokeWidth="0.8" opacity="0.5" />
          <rect x="4.5" y="2" width="5" height="2" rx="0.8" />
          <path d="M5.5 2 Q7 0.5 8.5 2" fill="none"
                stroke="currentColor" strokeWidth="1" strokeLinecap="round" />
          <ellipse cx="7" cy="9" rx="1.5" ry="2" opacity="0.7" />
        </svg>
        <span className="text-[11px] font-semibold text-neutral-400 tracking-wider uppercase">
          Lantern
        </span>
      </div>

      {/* ── Centre: library + document name + toolbar ───────────────────── */}
      <div
        data-tauri-drag-region
        className="flex-1 h-full flex items-center justify-center gap-3"
      >
        {tabs.length > 0 && (
          <button
            type="button"
            onClick={showLibrary}
            disabled={activeTab === null}
            aria-current={activeTab === null ? "page" : undefined}
            aria-label={t("titleBar.library")}
            title={t("titleBar.libraryHint")}
            className={`h-full px-2 flex items-center gap-1.5
                        text-[10px] uppercase tracking-wider
                        transition-colors focus:outline-none
                        focus-visible:ring-1 focus-visible:ring-accent
                        focus-visible:ring-inset select-none
                        ${activeTab === null
                          ? "text-accent"
                          : "text-neutral-400 hover:text-neutral-100"
                        }`}
          >
            <HomeIcon className="w-3 h-3" />
            {t("titleBar.library")}
          </button>
        )}
        {activeInfo && (
          <>
            {/* audit P2 #21: neutral-500 (~3.7:1) → neutral-400 (~5.5:1) so the
                doc-title clears WCAG AA on the title-bar surface. */}
            <span
              data-tauri-drag-region
              className="text-[11px] text-neutral-400 truncate max-w-[260px]"
              title={activeInfo.path ?? activeInfo.title}
            >
              {activeInfo.dirty && (
                <span className="text-accent mr-1" aria-label="unsaved changes">●</span>
              )}
              {activeInfo.title}
            </span>

            {/* audit P2 #21: Export button text neutral-500 → neutral-300 and
                full-strength accent ring on focus (was /60). */}
            <button
              onClick={handleExport}
              disabled={exporting}
              className="px-2 py-0.5 rounded text-[10px] text-neutral-300
                         border border-neutral-700/60
                         hover:border-neutral-500 hover:text-neutral-100
                         disabled:opacity-40 transition-colors focus:outline-none
                         focus-visible:ring-1 focus-visible:ring-accent select-none"
            >
              {exporting ? "Exporting…" : "Export…"}
            </button>
          </>
        )}
      </div>

      {/* ── Right: settings + window controls ────────────────────────────── */}
      <div className="flex items-center h-full shrink-0">
        <div className="relative h-full" ref={toolsRef}>
          {/* audit P2 #21/#25: bump trigger text to neutral-400 (~7:1 vs surface-1)
              and add focus-visible ring; trigger was previously relying on
              browser default which the dark surface hides. */}
          <button
            ref={toolsTriggerRef}
            type="button"
            onClick={() => setToolsOpen((open) => !open)}
            onKeyDown={handleTriggerKeyDown}
            aria-label={t("titleBar.tools")}
            aria-haspopup="menu"
            aria-expanded={toolsOpen}
            title={t("titleBar.tools")}
            className="h-full px-2.5 flex items-center gap-1.5
                       text-[10px] uppercase tracking-wider text-neutral-400
                       hover:text-neutral-100 transition-colors
                       focus:outline-none focus-visible:ring-1
                       focus-visible:ring-accent focus-visible:ring-inset"
          >
            <LinkIcon className="w-3 h-3" />
            {t("titleBar.tools")}
            <ChevronDownIcon className="w-2.5 h-2.5" />
          </button>
          {toolsOpen && (
            <div
              ref={toolsMenuRef}
              role="menu"
              aria-label={t("titleBar.tools")}
              onKeyDown={handleMenuKeyDown}
              className="absolute right-0 top-full mt-1 w-52 rounded-md border border-neutral-800
                         bg-surface-1 shadow-2xl overflow-hidden z-50"
            >
              <ToolMenuButton
                label={t("titleBar.tools.diff")}
                detail="Ctrl+Shift+D"
                disabled={!canCompareTabs}
                onClick={() => {
                  closeToolsMenu();
                  onCompareTabs?.();
                }}
              />
              <ToolMenuButton
                label={t("titleBar.tools.merge")}
                detail="Pick subtrees and merge"
                disabled={!canMergeDocuments}
                onClick={() => {
                  closeToolsMenu();
                  onMergeDocuments?.();
                }}
              />
              <ToolMenuButton
                label={t("titleBar.tools.deadLinks")}
                detail="Opt-in network check"
                disabled={!canCheckDeadLinks}
                onClick={() => {
                  closeToolsMenu();
                  onCheckDeadLinks?.();
                }}
              />
            </div>
          )}
        </div>
        {/* audit P2 #18: gear bumped from w-8 (32px) to w-10 (40px) to match
            window-control rhythm; focus-visible ring added for keyboard nav. */}
        {onSettings && (
          <button
            onClick={onSettings}
            aria-label={t("titleBar.settings")}
            title={`${t("titleBar.settings")} (Ctrl+,)`}
            className="w-10 h-full flex items-center justify-center
                       text-neutral-400 hover:text-neutral-100
                       transition-colors focus:outline-none
                       focus-visible:ring-1 focus-visible:ring-accent
                       focus-visible:ring-inset"
          >
            <GearIcon className="w-3.5 h-3.5" />
          </button>
        )}
        <WinBtn label={t("titleBar.minimize")} onClick={() => appWindow.minimize()}>
          <MinimiseIcon />
        </WinBtn>
        <WinBtn label={t("titleBar.maximize")} onClick={() => appWindow.toggleMaximize()}>
          <MaximiseIcon />
        </WinBtn>
        <WinBtn label={t("titleBar.close")} onClick={() => appWindow.close()} danger>
          <CloseIcon />
        </WinBtn>
      </div>
    </header>
  );
}

function ToolMenuButton({
  label,
  detail,
  disabled,
  onClick,
}: {
  label: string;
  detail: string;
  disabled: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      onClick={onClick}
      disabled={disabled}
      className="w-full text-left px-3 py-2 border-b border-neutral-800 last:border-b-0
                 hover:bg-surface-2 disabled:hover:bg-transparent disabled:opacity-40
                 transition-colors focus:outline-none focus-visible:bg-surface-2"
    >
      <div className="text-xs text-neutral-200">{label}</div>
      <div className="text-[10px] text-neutral-500">{detail}</div>
    </button>
  );
}

// ---------------------------------------------------------------------------
// Window control button
// ---------------------------------------------------------------------------

function WinBtn({
  label,
  onClick,
  danger = false,
  children,
}: {
  label:    string;
  onClick:  () => void;
  danger?:  boolean;
  children: React.ReactNode;
}) {
  // audit P2 #18: window controls bumped from w-10 (40px) to w-12 (48px) so
  // they comfortably exceed the NFR-A-5 32x32 minimum.  Title-bar height is 32 px
  // so vertical dimension already meets the floor; widening lifts the hit area
  // from 1280 to 1536 sq.px and lines up with native Windows controls visually.
  // Focus-visible ring added so keyboard users can see the active control.
  return (
    <button
      aria-label={label}
      onClick={onClick}
      className={`w-12 h-full flex items-center justify-center
                  text-neutral-400 transition-colors
                  focus:outline-none focus-visible:ring-1
                  focus-visible:ring-accent focus-visible:ring-inset
                  ${danger
                    ? "hover:bg-danger hover:text-white"
                    : "hover:bg-neutral-700/60 hover:text-neutral-100"
                  }`}
    >
      {children}
    </button>
  );
}

// ---------------------------------------------------------------------------
// SVG icons for window controls (14×10 canvas)
// ---------------------------------------------------------------------------

function MinimiseIcon() {
  return (
    <svg viewBox="0 0 14 10" className="w-3.5 h-2.5" fill="none" aria-hidden>
      <line x1="2" y1="5" x2="12" y2="5"
            stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}

function MaximiseIcon() {
  return (
    <svg viewBox="0 0 14 10" className="w-3.5 h-2.5" fill="none" aria-hidden>
      <rect x="2" y="1" width="10" height="8" rx="1"
            stroke="currentColor" strokeWidth="1.2" />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg viewBox="0 0 14 10" className="w-3.5 h-2.5" fill="none" aria-hidden>
      <path d="M3 1.5 L11 8.5 M11 1.5 L3 8.5"
            stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" />
    </svg>
  );
}
