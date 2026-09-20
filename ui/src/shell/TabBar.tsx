/**
 * Tab bar: one tab per open document.
 *
 * Keyboard contract (NFR-A-1):
 *  - Tab list is a proper WAI-ARIA `tablist` so screen readers announce the
 *    active tab and the count.
 *  - Arrow Left / Right move between tabs (with wraparound), Home / End jump
 *    to the first / last tab.  Activating a tab via the arrow keys also moves
 *    focus to it; pressing Enter/Space on a tab activates it.
 *  - Each tab carries `aria-controls` pointing at the panel id used by the
 *    document workspace, so `useFocusVisible`-style assistive tech can pair
 *    them up even though the workspace itself is rendered by a sibling tree.
 *
 * Mouse / pointer extras (v0.0.11 QoL):
 *  - Middle-click (mouse button 1) on a tab closes it.
 *  - Right-click on a tab opens a `ContextMenu` with close-tab,
 *    close-others, close-to-right, close-all.
 */

import { useRef, useState } from "react";

import { useDocuments } from "../state/documents";
import { useT } from "../i18n/I18nProvider";
import { XIcon } from "../components/Icons";
import { ContextMenu, type ContextMenuItem } from "../components/ContextMenu";
import type { TabId } from "../ipc/types";

/** Stable id used for the corresponding document workspace `tabpanel`. */
export function tabPanelId(tabId: TabId): string {
  return `tabpanel-${tabId}`;
}

/** Stable id used for the tab control itself (so a panel can mirror it). */
export function tabControlId(tabId: TabId): string {
  return `tab-${tabId}`;
}

interface ContextMenuState {
  x: number;
  y: number;
  tabId: TabId;
}

export default function TabBar() {
  const t = useT();
  const { tabs, activeTab, setActiveTab, closeTab } = useDocuments();
  // One ref per tab control so we can move focus on arrow-key navigation.
  const tabRefs = useRef<Map<TabId, HTMLButtonElement | null>>(new Map());
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);

  const focusTab = (id: TabId) => {
    const node = tabRefs.current.get(id);
    if (node) node.focus();
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>, currentId: TabId) => {
    if (tabs.length === 0) return;
    const idx = tabs.findIndex((t) => t.id === currentId);
    if (idx < 0) return;

    let nextIndex: number | null = null;
    switch (event.key) {
      case "ArrowRight":
        nextIndex = (idx + 1) % tabs.length;
        break;
      case "ArrowLeft":
        nextIndex = (idx - 1 + tabs.length) % tabs.length;
        break;
      case "Home":
        nextIndex = 0;
        break;
      case "End":
        nextIndex = tabs.length - 1;
        break;
      default:
        return;
    }

    if (nextIndex === null) return;
    event.preventDefault();
    const nextTab = tabs[nextIndex];
    setActiveTab(nextTab.id);
    // Defer focus until React has flushed the active-state class change so
    // we don't fight a focus reset.
    requestAnimationFrame(() => focusTab(nextTab.id));
  };

  // ── Close helpers used by context-menu items.  We loop over `closeTab`
  //    rather than introducing a `closeMany` IPC because the close call
  //    is already cheap and serial closure means a single refresh pass
  //    runs for each one (preserves the existing redraw semantics). ────
  const closeOthers = (keepId: TabId) => {
    for (const tab of tabs) {
      if (tab.id !== keepId) closeTab(tab.id);
    }
  };

  const closeToRight = (afterId: TabId) => {
    const idx = tabs.findIndex((t) => t.id === afterId);
    if (idx < 0) return;
    for (const tab of tabs.slice(idx + 1)) closeTab(tab.id);
  };

  const closeAll = () => {
    for (const tab of tabs) closeTab(tab.id);
  };

  const buildContextItems = (tabId: TabId): ContextMenuItem[] => {
    const idx = tabs.findIndex((t) => t.id === tabId);
    const isRightmost = idx === tabs.length - 1;
    const onlyOne = tabs.length <= 1;
    return [
      {
        id: "close",
        label: t("tabBar.context.closeTab"),
        onSelect: () => closeTab(tabId),
      },
      {
        id: "close-others",
        label: t("tabBar.context.closeOthers"),
        disabled: onlyOne,
        onSelect: () => closeOthers(tabId),
      },
      {
        id: "close-right",
        label: t("tabBar.context.closeToRight"),
        disabled: isRightmost,
        onSelect: () => closeToRight(tabId),
      },
      { id: "sep", separator: true },
      {
        id: "close-all",
        label: t("tabBar.context.closeAll"),
        onSelect: () => closeAll(),
      },
    ];
  };

  return (
    <div
      role="tablist"
      aria-label="Open documents"
      aria-orientation="horizontal"
      className="h-9 flex items-end px-2 gap-0.5 bg-surface-0
                 border-b border-neutral-800 shrink-0 overflow-x-auto"
    >
      {tabs.map((tab, idx) => {
        const active = tab.id === activeTab;
        // audit P2 #21: inactive tab text neutral-500 (~3.7:1) →
        // neutral-400 (~5.5:1) so non-active tabs clear AA on surface-0.
        return (
          <div
            key={tab.id}
            className={`group flex items-center h-8 rounded-t text-xs
                        select-none transition-colors
                        ${active
                ? "bg-surface-1 text-neutral-100 border border-b-0 border-neutral-800"
                : "text-neutral-400 hover:text-neutral-100 hover:bg-surface-2"
              }`}
          >
            <button
              ref={(node) => {
                if (node) tabRefs.current.set(tab.id, node);
                else tabRefs.current.delete(tab.id);
              }}
              role="tab"
              type="button"
              id={tabControlId(tab.id)}
              aria-selected={active}
              aria-controls={tabPanelId(tab.id)}
              tabIndex={active || (activeTab === null && idx === 0) ? 0 : -1}
              onClick={() => setActiveTab(tab.id)}
              onAuxClick={(e) => {
                if (e.button === 1) {
                  e.preventDefault();
                  closeTab(tab.id);
                }
              }}
              onContextMenu={(e) => {
                e.preventDefault();
                setContextMenu({ x: e.clientX, y: e.clientY, tabId: tab.id });
              }}
              onKeyDown={(e) => handleKeyDown(e, tab.id)}
              className="flex items-center gap-2 h-8 pl-3 pr-1 cursor-pointer
                         focus:outline-none focus-visible:ring-1
                         focus-visible:ring-accent rounded-t"
            >
              <span className="max-w-[160px] truncate">
                {tab.dirty && (
                  <span className="mr-1 text-accent" aria-hidden>●</span>
                )}
                {tab.title || t("tabBar.untitled")}
              </span>
            </button>
            {/* Close button (audit P2 #19): hit area bumped from 24x24 to 32x24
                (the tab strip is only 32 px tall so we can't go taller, but the
                horizontal axis matches the NFR-A-5 floor and the active +
                hover affordances make it clearly clickable). */}
            <button
              type="button"
              aria-label={`${t("tabBar.closeTab")}: ${tab.title || t("tabBar.untitled")}`}
              onClick={(e) => {
                e.stopPropagation();
                closeTab(tab.id);
              }}
              tabIndex={-1}
              className={`inline-flex items-center justify-center
                          min-w-[32px] min-h-[24px] mr-1 rounded
                          text-neutral-400 hover:text-neutral-100
                          hover:bg-neutral-700/50 transition-all
                          focus:outline-none focus-visible:ring-1
                          focus-visible:ring-accent
                          ${active ? "opacity-60 hover:opacity-100" : "opacity-0 group-hover:opacity-100 focus-visible:opacity-100"}`}
            >
              <XIcon className="w-2.5 h-2.5" />
            </button>
          </div>
        );
      })}

      {contextMenu !== null && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          ariaLabel="Tab actions"
          items={buildContextItems(contextMenu.tabId)}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
}
