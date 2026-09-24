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

import { useEffect, useMemo, useState } from "react";

import { useDocuments } from "./state/documents";
import { useFileActions } from "./state/fileActions";
import { ipc } from "./ipc";
import { applyTheme, resolveTheme, useTheme } from "./hooks/useTheme";
import { useT } from "./i18n/I18nProvider";
import ErrorBoundary from "./components/ErrorBoundary";
import TitleBar from "./shell/TitleBar";
import TabBar, { tabControlId, tabPanelId } from "./shell/TabBar";
import StatusBar from "./shell/StatusBar";
import LibraryHome from "./shell/LibraryHome";
import CloseGuardDialog from "./shell/CloseGuardDialog";
import TreePane from "./panes/TreePane";
import ListPane from "./panes/ListPane";
import DetailPane from "./panes/DetailPane";
import { SettingsModal } from "./components/SettingsModal";
import { DiffModal } from "./components/DiffModal";
import { DeadLinkModal } from "./components/DeadLinkModal";
import { MergePickerModal } from "./components/MergePickerModal";
import { CommandPalette } from "./components/CommandPalette";
import { ReviewPane } from "./components/ReviewPane";
import {
  requestRunPass,
  requestSearchFocus,
  type CommandId,
  type PaletteCommand,
} from "./components/commandPalette";
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
  const t = useT();
  const {
    tabs,
    activeTab,
    showLibrary,
    refreshTabs,
    setActiveTab,
    isSearchMode,
    clearSearch,
    goBack,
    goForward,
    review,
  } = useDocuments();
  const activeReview = review && review.tabId === activeTab ? review : null;

  const [settingsOpen, setSettingsOpen] = useState(false);
  const [diffOpen, setDiffOpen] = useState(false);
  const [deadLinkOpen, setDeadLinkOpen] = useState(false);
  const [mergeOpen, setMergeOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [themeNonce, setThemeNonce] = useState(0);
  const [dropping, setDropping] = useState(false);
  const { openPaths, pickAndOpen, saveCopy, undo, redo } = useFileActions();
  const [density, setDensity] = useState<"compact" | "comfortable">("compact");

  // Theme: read on mount, re-read whenever the settings modal closes or
  // the command-palette toggle writes a new theme.  The hook also
  // subscribes to OS prefers-color-scheme changes when the user setting
  // is "system".
  useTheme(settingsOpen || themeNonce);

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

  // Closing the window (Alt+F4, the OS, the title-bar button) with edited
  // tabs asks first; see CloseGuardDialog.  Outside Tauri this is a no-op.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    import("@tauri-apps/api/window")
      .then(({ getCurrentWindow }) =>
        getCurrentWindow().onCloseRequested((event) => {
          const { tabs: open, requestClose } = useDocuments.getState();
          if (open.some((tab) => tab.dirty)) {
            event.preventDefault();
            void requestClose(open.map((tab) => tab.id), { closeWindow: true });
          }
        }),
      )
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // Not running inside Tauri.
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  // Files dropped anywhere on the window open as volumes.  Tauri delivers
  // real paths through the webview drag-drop event (plain HTML5 drops only
  // expose File objects); outside Tauri the import rejects and this is a no-op.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    import("@tauri-apps/api/webview")
      .then(({ getCurrentWebview }) =>
        getCurrentWebview().onDragDropEvent((event) => {
          const { type } = event.payload;
          if (type === "enter" || type === "over") setDropping(true);
          else setDropping(false);
          if (type === "drop") void openPaths(event.payload.paths);
        }),
      )
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // Not running inside Tauri.
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [openPaths]);

  const toggleTheme = async () => {
    try {
      const s = await ipc.getSettings();
      const next = resolveTheme(s.theme) === "dark" ? "light" : "dark";
      await ipc.updateSettings({ ...s, theme: next });
      applyTheme(next);
      setThemeNonce((n) => n + 1);
    } catch {
      // not in Tauri context
    }
  };

  const runPaletteCommand = (id: CommandId) => {
    setPaletteOpen(false);
    // Defer until the palette focus trap has restored the prior element,
    // so the destination (settings dialog, search input, …) keeps focus.
    window.setTimeout(() => {
      switch (id) {
        case "open_file":
          void pickAndOpen();
          break;
        case "settings":
          setSettingsOpen(true);
          break;
        case "save_copy":
          void saveCopy();
          break;
        case "save_copy_as":
          void saveCopy({ saveAs: true });
          break;
        case "run_pass":
          requestRunPass();
          break;
        case "search_focus":
          requestSearchFocus();
          break;
        case "compare_tabs":
          setDiffOpen(true);
          break;
        case "check_dead_links":
          setDeadLinkOpen(true);
          break;
        case "merge_documents":
          setMergeOpen(true);
          break;
        case "toggle_theme":
          void toggleTheme();
          break;
      }
    }, 0);
  };

  const paletteCommands: PaletteCommand[] = useMemo(
    () => [
      { id: "open_file", label: t("commandPalette.openFile"), shortcut: "Ctrl+O", enabled: true },
      { id: "settings", label: t("titleBar.settings"), shortcut: "Ctrl+,", enabled: true },
      { id: "save_copy", label: t("commandPalette.saveCopy"), shortcut: "Ctrl+S", enabled: activeTab !== null },
      { id: "save_copy_as", label: t("commandPalette.saveCopyAs"), shortcut: "Ctrl+Shift+S", enabled: activeTab !== null },
      { id: "run_pass", label: t("commandPalette.runPass"), shortcut: "Ctrl+R", enabled: activeTab !== null },
      { id: "search_focus", label: t("commandPalette.search"), shortcut: "Ctrl+F", enabled: activeTab !== null },
      {
        id: "compare_tabs",
        label: t("titleBar.tools.diff"),
        shortcut: "Ctrl+Shift+D",
        enabled: tabs.length >= 2,
      },
      {
        id: "check_dead_links",
        label: t("titleBar.tools.deadLinks"),
        enabled: activeTab !== null,
      },
      {
        id: "merge_documents",
        label: t("titleBar.tools.merge"),
        shortcut: "Ctrl+M",
        enabled: tabs.length >= 1,
      },
      { id: "toggle_theme", label: t("commandPalette.toggleTheme"), enabled: true },
    ],
    [t, activeTab, tabs.length],
  );

  // Global keyboard shortcuts
  useEffect(() => {
    const handler = async (e: KeyboardEvent) => {
      // Ctrl+K / Cmd+K → command palette (toggle)
      if (
        (e.ctrlKey || e.metaKey) &&
        !e.shiftKey &&
        !e.altKey &&
        (e.key === "k" || e.key === "K")
      ) {
        e.preventDefault();
        setPaletteOpen((open) => !open);
        return;
      }

      // Ctrl+, → settings
      if (e.ctrlKey && !e.shiftKey && e.key === ",") {
        e.preventDefault();
        setSettingsOpen(true);
        return;
      }

      const key = e.key.toLowerCase();

      // Ctrl+W → close the active tab; Ctrl+Shift+W → close all (edited
      // tabs are confirmed by the close guard)
      if (e.ctrlKey && key === "w") {
        e.preventDefault();
        const { tabs: open, activeTab: current, requestClose } = useDocuments.getState();
        if (e.shiftKey) void requestClose(open.map((tab) => tab.id));
        else if (current !== null) void requestClose([current]);
        return;
      }

      // Ctrl+Tab / Ctrl+Shift+Tab → next / previous tab
      if (e.ctrlKey && e.key === "Tab") {
        const { tabs: open, activeTab: current } = useDocuments.getState();
        if (open.length === 0) return;
        e.preventDefault();
        const index = open.findIndex((tab) => tab.id === current);
        const step = e.shiftKey ? open.length - 1 : 1;
        await setActiveTab(open[(Math.max(index, 0) + step) % open.length].id);
        return;
      }

      // Ctrl+F → search this document; Ctrl+R → run the selected rule set
      if (e.ctrlKey && !e.shiftKey && (key === "f" || key === "r")) {
        if (activeTab === null) return;
        e.preventDefault();
        if (key === "f") requestSearchFocus();
        else requestRunPass();
        return;
      }

      // Ctrl+M → merge documents
      if (e.ctrlKey && !e.shiftKey && key === "m") {
        if (tabs.length === 0) return;
        e.preventDefault();
        setMergeOpen(true);
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

      // Ctrl+O → open file(s)
      if (e.ctrlKey && !e.shiftKey && (e.key === "o" || e.key === "O")) {
        e.preventDefault();
        await pickAndOpen();
        return;
      }

      // Ctrl+S → save a copy (asks where the first time); Ctrl+Shift+S → always ask
      if (e.ctrlKey && (e.key === "s" || e.key === "S")) {
        if (activeTab === null) return;
        e.preventDefault();
        await saveCopy({ saveAs: e.shiftKey });
        return;
      }

      // Ctrl+Z → undo; Ctrl+Y or Ctrl+Shift+Z → redo.  Text fields keep
      // their own undo.
      if (e.ctrlKey && (e.key === "z" || e.key === "Z" || e.key === "y" || e.key === "Y")) {
        if (activeTab === null || isEditableTarget(e.target)) return;
        e.preventDefault();
        const isRedo = e.key === "y" || e.key === "Y" || e.shiftKey;
        await (isRedo ? redo() : undo());
        return;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [showLibrary, activeTab, tabs.length, setActiveTab, pickAndOpen, saveCopy, undo, redo]);

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
    <div className="flex flex-col h-screen bg-surface-0 text-ink overflow-hidden">
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

          {activeReview ? (
            /* A pass is awaiting a decision: the review takes the list and
               detail columns so full URLs and folder paths fit. */
            <ReviewPane review={activeReview} />
          ) : (
            <>
              {/* Centre: item list */}
              <section className="flex-1 min-w-0 flex flex-col border-r border-neutral-800">
                <ListPane />
              </section>

              {/* Right: item detail */}
              <aside className="w-72 shrink-0 flex flex-col">
                <DetailPane />
              </aside>
            </>
          )}
        </main>
      ) : (
        <LibraryHome onOpenPaths={openPaths} onPickFiles={pickAndOpen} />
      )}

      {/* Status bar */}
      <StatusBar />

      {/* Settings modal */}
      <SettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
      />

      <CommandPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        commands={paletteCommands}
        onRun={runPaletteCommand}
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

      <CloseGuardDialog />

      {/* Drop target feedback while files are dragged over the window */}
      {dropping && (
        <div
          aria-hidden
          className="fixed inset-2 z-[60] pointer-events-none rounded-xl border-2 border-dashed
                     border-accent/70 bg-accent/10 flex items-center justify-center animate-fade-in"
        >
          <p className="px-4 py-2 rounded-lg bg-surface-1/90 text-sm font-medium text-neutral-100 shadow-lg">
            {t("file.dropToOpen")}
          </p>
        </div>
      )}

      {/* Global toast / notification surface (v0.0.11 QoL slice 1).  Mounted
          last so its z-index 70 stack reliably overlays any modal that opens
          underneath it. */}
      <Toaster />
    </div>
    </ErrorBoundary>
  );
}
