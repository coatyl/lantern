/**
 * Root application component.
 *
 * Layout (02-design-doc.md §3):
 *
 *   ┌─────────────────────────────────────────────────────┐
 *   │  TitleBar  (drag region + window controls)          │
 *   ├──────────────────────────────────────────────────── │
 *   │  TabBar    (one tab per open document)              │
 *   ├──────────────────────────────────────────────────── │
 *   │  TreePane  │  ListPane              │  DetailPane   │
 *   │  (folders) │  (bookmarks in folder) │  (item detail)│
 *   ├──────────────────────────────────────────────────── │
 *   │  StatusBar (stats · network · dirty indicator)      │
 *   └─────────────────────────────────────────────────────┘
 */

import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import { useDocuments } from "./state/documents";
import { ipc } from "./ipc";
import { useTheme } from "./hooks/useTheme";
import ErrorBoundary from "./components/ErrorBoundary";
import TitleBar from "./shell/TitleBar";
import TabBar, { tabControlId, tabPanelId } from "./shell/TabBar";
import StatusBar from "./shell/StatusBar";
import LibraryHome from "./shell/LibraryHome";
import TreePane from "./panes/TreePane";
import ListPane from "./panes/ListPane";
import DetailPane from "./panes/DetailPane";
import { SettingsModal } from "./components/SettingsModal";
import { DiffModal } from "./components/DiffModal";
import { DeadLinkModal } from "./components/DeadLinkModal";
import { MergePickerModal } from "./components/MergePickerModal";
import Toaster from "./components/Toast";

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return Boolean(
    target.closest(
      'input, textarea, select, [contenteditable="true"], [contenteditable=""], [role="textbox"]',
    ),
  );
}

export default function App() {
  const {
    tabs,
    activeTab,
    openFile,
    showLibrary,
    refreshTabs,
    setActiveTab,
    refreshTree,
    refreshList,
    isSearchMode,
    clearSearch,
    goBack,
    goForward,
  } = useDocuments();

  const [settingsOpen, setSettingsOpen] = useState(false);
  const [diffOpen, setDiffOpen] = useState(false);
  const [deadLinkOpen, setDeadLinkOpen] = useState(false);
  const [mergeOpen, setMergeOpen] = useState(false);
  const [density, setDensity] = useState<"compact" | "comfortable">("compact");

  // Theme: read on mount, re-read whenever the settings modal closes.  The
  // hook also subscribes to OS prefers-color-scheme changes when the user
  // setting is "system".
  useTheme(settingsOpen);

  // Pull list density from settings on mount and again whenever the settings
  // modal closes, so toggling the option updates the layout immediately.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const s = await ipc.getSettings();
        if (!cancelled) setDensity(s.list_density);
      } catch {
        // not in tauri context, keep default
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [settingsOpen]);

  useEffect(() => {
    document.documentElement.setAttribute("data-density", density);
  }, [density]);

  // Hydrate already-open documents on startup so the three-pane workspace
  // mounts without an explicit open action. On a normal cold start the
  // backend reports no open tabs and we fall through to the library home;
  // when documents are already present (e.g. the pre-opened fixture used by
  // browser-preview / E2E mode) the first tab is activated automatically.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        await refreshTabs();
        const { tabs: openTabs, activeTab: current } = useDocuments.getState();
        if (!cancelled && current === null && openTabs.length > 0) {
          await setActiveTab(openTabs[0].id);
        }
      } catch {
        // Not in a Tauri context (or IPC unavailable); stay on the library home.
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Global keyboard shortcuts
  useEffect(() => {
    const handler = async (e: KeyboardEvent) => {
      // Ctrl+, → settings
      if (e.ctrlKey && !e.shiftKey && e.key === ",") {
        e.preventDefault();
        setSettingsOpen(true);
        return;
      }

      // Ctrl+Shift+D → compare two tabs
      if (e.ctrlKey && e.shiftKey && (e.key === "d" || e.key === "D")) {
        e.preventDefault();
        setDiffOpen(true);
        return;
      }

      // Ctrl+Shift+L → library home (tabs stay open)
      if (e.ctrlKey && e.shiftKey && (e.key === "l" || e.key === "L")) {
        e.preventDefault();
        showLibrary();
        return;
      }

      // Ctrl+O → open file
      if (e.ctrlKey && !e.shiftKey && e.key === "o") {
        e.preventDefault();
        const selected = await open({
          filters: [{ name: "Bookmark files", extensions: ["html", "htm"] }],
          multiple: false,
        });
        if (typeof selected === "string") {
          await openFile(selected);
        }
        return;
      }

      // Ctrl+S → save in place (export to the original file path)
      if (e.ctrlKey && !e.shiftKey && e.key === "s") {
        if (activeTab === null) return;
        e.preventDefault();
        const info = tabs.find((t) => t.id === activeTab);
        if (!info?.path) return; // no known path, ignore (user should use Export…)
        try {
          await ipc.export(activeTab, { kind: "whole_document" }, info.path);
          await refreshTree(); // dirty flag cleared
        } catch {
          // write error, ignore silently for now (toasts in v0.0.2)
        }
        return;
      }

      // Ctrl+Z → undo
      if (e.ctrlKey && !e.shiftKey && e.key === "z") {
        if (activeTab === null) return;
        e.preventDefault();
        try {
          await ipc.undo(activeTab);
          await Promise.all([refreshTree(), refreshList()]);
        } catch {
          // undo unavailable, ignore
        }
        return;
      }

      // Ctrl+Y  or  Ctrl+Shift+Z → redo
      if (e.ctrlKey && (e.key === "y" || (e.shiftKey && e.key === "z"))) {
        if (activeTab === null) return;
        e.preventDefault();
        try {
          await ipc.redo(activeTab);
          await Promise.all([refreshTree(), refreshList()]);
        } catch {
          // redo unavailable, ignore
        }
        return;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [openFile, showLibrary, activeTab, tabs, refreshTree, refreshList]);

  useEffect(() => {
    const preventBrowserButtons = (e: MouseEvent) => {
      if (e.button === 3 || e.button === 4) {
        e.preventDefault();
      }
    };

    const handleMouseNavigation = async (e: MouseEvent) => {
      if (activeTab === null || isEditableTarget(e.target)) return;

      if (e.button === 3) {
        e.preventDefault();
        if (isSearchMode) {
          clearSearch();
          return;
        }
        await goBack();
      } else if (e.button === 4) {
        e.preventDefault();
        await goForward();
      }
    };

    window.addEventListener("mousedown", preventBrowserButtons);
    window.addEventListener("mouseup", handleMouseNavigation);
    return () => {
      window.removeEventListener("mousedown", preventBrowserButtons);
      window.removeEventListener("mouseup", handleMouseNavigation);
    };
  }, [activeTab, isSearchMode, clearSearch, goBack, goForward]);

  return (
    <ErrorBoundary>
    <div className="flex flex-col h-screen bg-surface-0 text-neutral-100 overflow-hidden">
      {/* Custom title bar (window is frameless) */}
      <TitleBar
        onSettings={() => setSettingsOpen(true)}
        onCompareTabs={() => setDiffOpen(true)}
        onCheckDeadLinks={() => setDeadLinkOpen(true)}
        onMergeDocuments={() => setMergeOpen(true)}
        canCompareTabs={tabs.length >= 2}
        canCheckDeadLinks={activeTab !== null}
        canMergeDocuments={tabs.length >= 1}
      />

      {/* Tab bar */}
      {tabs.length > 0 && <TabBar />}

      {/* Three-pane workspace: shown when a volume tab is focused.
          Library home is the stacks; this pane is inside a volume. */}
      {activeTab !== null ? (
        <main
          className="flex flex-1 min-h-0"
          role="tabpanel"
          id={tabPanelId(activeTab)}
          aria-labelledby={tabControlId(activeTab)}
        >
          {/* Left: folder tree */}
          <aside className="w-56 shrink-0 border-r border-neutral-800 flex flex-col">
            <TreePane />
          </aside>

          {/* Centre: item list */}
          <section className="flex-1 min-w-0 flex flex-col border-r border-neutral-800">
            <ListPane />
          </section>

          {/* Right: item detail */}
          <aside className="w-72 shrink-0 flex flex-col">
            <DetailPane />
          </aside>
        </main>
      ) : (
        <LibraryHome onOpen={openFile} />
      )}

      {/* Status bar */}
      <StatusBar />

      {/* Settings modal */}
      <SettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
      />

      {/* Compare tabs modal: opened by Ctrl+Shift+D */}
      <DiffModal
        open={diffOpen}
        tabs={tabs}
        initialLeft={activeTab}
        initialRight={tabs.find((t) => t.id !== activeTab)?.id ?? null}
        onClose={() => setDiffOpen(false)}
      />

      <DeadLinkModal
        open={deadLinkOpen}
        tabId={activeTab}
        tabTitle={tabs.find((t) => t.id === activeTab)?.title ?? null}
        onClose={() => setDeadLinkOpen(false)}
        onOpenSettings={() => {
          setDeadLinkOpen(false);
          setSettingsOpen(true);
        }}
      />

      {/* Cross-document merge picker (v0.0.7, ADR-0009) */}
      <MergePickerModal
        open={mergeOpen}
        tabs={tabs}
        onClose={() => setMergeOpen(false)}
        onMerged={async (newTabId) => {
          await refreshTabs();
          await setActiveTab(newTabId);
        }}
      />

      {/* Global toast / notification surface (v0.0.11 QoL slice 1).  Mounted
          last so its z-index 70 stack reliably overlays any modal that opens
          underneath it. */}
      <Toaster />
    </div>
    </ErrorBoundary>
  );
}
