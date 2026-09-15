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
  }, [openFile, activeTab, tabs, refreshTree, refreshList]);

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

      {/* Three-pane workspace: only shown when a document is open */}
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
        <WelcomeScreen onOpen={openFile} />
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

// ---------------------------------------------------------------------------
// Welcome screen (shown when no document is open)
// ---------------------------------------------------------------------------

/**
 * Full-size lantern SVG for the welcome screen.
 * Same proportions as the TitleBar micro-icon but rendered at 72 × 90 px.
 */
function LanternLogo() {
  return (
    <svg
      viewBox="0 0 28 36"
      className="w-16 h-20 text-accent drop-shadow-[0_0_18px_rgb(var(--accent)/0.35)]"
      fill="currentColor"
      aria-hidden
    >
      {/* Body: filled with low opacity */}
      <rect x="5" y="7" width="18" height="22" rx="3" opacity="0.18" />
      {/* Body: outline */}
      <rect x="5" y="7" width="18" height="22" rx="3"
            fill="none" stroke="currentColor" strokeWidth="1.8" />
      {/* Horizontal divider */}
      <line x1="5" y1="18" x2="23" y2="18"
            stroke="currentColor" strokeWidth="1.1" opacity="0.45" />
      {/* Vertical divider */}
      <line x1="14" y1="7" x2="14" y2="29"
            stroke="currentColor" strokeWidth="1.1" opacity="0.45" />
      {/* Cap */}
      <rect x="9" y="4" width="10" height="4" rx="1.5" />
      {/* Hook / arch */}
      <path d="M10 4 Q14 0.5 18 4"
            fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
      {/* Flame / glow centre */}
      <ellipse cx="14" cy="18" rx="3.5" ry="4.5" opacity="0.65" />
    </svg>
  );
}

function WelcomeScreen({ onOpen }: { onOpen: (path: string) => Promise<void> }) {
  const { refreshTabs, setActiveTab } = useDocuments();
  const [recentFiles, setRecentFiles] = useState<string[]>([]);
  const [recoveryPaths, setRecoveryPaths] = useState<string[]>([]);
  const [recoveryMessage, setRecoveryMessage] = useState<string | null>(null);
  const [restoringSession, setRestoringSession] = useState(false);
  const [clearingRecent, setClearingRecent] = useState(false);

  const loadWelcomeData = async () => {
    try {
      const [recent, recovery] = await Promise.all([
        ipc.listRecentFiles(),
        ipc.getRecoveryState(),
      ]);
      setRecentFiles(recent);
      setRecoveryPaths(recovery.paths);
    } catch {
      // not in Tauri context
    }
  };

  useEffect(() => {
    loadWelcomeData();
  }, []);

  const handleOpen = async () => {
    const selected = await open({
      filters: [{ name: "Bookmark files", extensions: ["html", "htm"] }],
      multiple: false,
    });
    if (typeof selected === "string") {
      await onOpen(selected);
    }
  };

  const handleRestoreSession = async () => {
    setRestoringSession(true);
    setRecoveryMessage(null);
    try {
      const report = await ipc.restoreRecoverySession();
      await refreshTabs();
      setRecoveryPaths([]);

      if (report.restored_tab_ids.length > 0) {
        await setActiveTab(report.restored_tab_ids[0]);
      }

      if (report.failed_paths.length > 0) {
        setRecoveryMessage(
          `Restored ${report.restored_paths.length} file${report.restored_paths.length !== 1 ? "s" : ""}; ` +
          `${report.failed_paths.length} could not be reopened.`,
        );
      }
      await loadWelcomeData();
    } catch {
      setRecoveryMessage("Could not restore the previous session.");
    } finally {
      setRestoringSession(false);
    }
  };

  const handleDismissRecovery = async () => {
    try {
      await ipc.dismissRecoverySession();
      setRecoveryPaths([]);
      setRecoveryMessage(null);
    } catch {
      // ignore
    }
  };

  const handleClearRecent = async () => {
    setClearingRecent(true);
    try {
      await ipc.clearRecentFiles();
      setRecentFiles([]);
    } catch {
      // ignore
    } finally {
      setClearingRecent(false);
    }
  };

  return (
    <div className="flex-1 flex flex-col items-center justify-center gap-6 text-neutral-400">
      {/* Lantern SVG logo */}
      <LanternLogo />

      <div className="text-center">
        <h1 className="text-xl font-semibold text-neutral-200 mb-1 tracking-wide">Lantern</h1>
        <p className="text-sm text-neutral-500">Open a bookmark file to get started.</p>
      </div>

      <button
        onClick={handleOpen}
        className="px-5 py-2 rounded-md bg-accent hover:bg-accent-hover text-neutral-950
                   font-semibold text-sm transition-colors focus:outline-none
                   focus-visible:ring-2 focus-visible:ring-accent/70"
      >
        Open file…
      </button>

      {recoveryPaths.length > 0 && (
        <div className="w-full max-w-lg rounded-lg border border-accent/20 bg-surface-2/80 px-4 py-3">
          <div className="flex items-center justify-between gap-3">
            <div>
              <p className="text-xs font-semibold uppercase tracking-wider text-accent/80">
                Recover Previous Session
              </p>
              <p className="mt-1 text-xs text-neutral-500">
                Lantern did not shut down cleanly last time. Reopen the previous files?
              </p>
            </div>
            <div className="flex gap-2 shrink-0">
              <button
                onClick={handleDismissRecovery}
                disabled={restoringSession}
                className="px-3 py-1.5 rounded border border-neutral-700 text-xs text-neutral-400
                           hover:text-neutral-200 hover:border-neutral-500 transition-colors
                           disabled:opacity-40"
              >
                Dismiss
              </button>
              <button
                onClick={handleRestoreSession}
                disabled={restoringSession}
                className="px-3 py-1.5 rounded bg-accent text-neutral-950 text-xs font-semibold
                           hover:bg-accent-hover transition-colors disabled:opacity-40"
              >
                {restoringSession ? "Restoring…" : "Restore"}
              </button>
            </div>
          </div>
          <div className="mt-3 space-y-1.5">
            {recoveryPaths.slice(0, 5).map((path) => (
              <div key={path} className="text-[11px] text-neutral-500 break-all">
                {path}
              </div>
            ))}
            {recoveryPaths.length > 5 && (
              <div className="text-[11px] text-neutral-600">
                +{recoveryPaths.length - 5} more file{recoveryPaths.length - 5 !== 1 ? "s" : ""}
              </div>
            )}
          </div>
        </div>
      )}

      {recoveryMessage && (
        <p className="text-xs text-neutral-500">{recoveryMessage}</p>
      )}

      {/* Recent files */}
      {recentFiles.length > 0 && (
        <div className="flex flex-col items-center gap-0.5 w-full max-w-sm mt-2">
          <div className="w-full flex items-center justify-between mb-2">
            <p className="text-[10px] uppercase tracking-wider text-neutral-700">Recent</p>
            <button
              onClick={handleClearRecent}
              disabled={clearingRecent}
              className="text-[10px] text-neutral-600 hover:text-neutral-300 transition-colors
                         disabled:opacity-40"
            >
              {clearingRecent ? "Clearing…" : "Clear"}
            </button>
          </div>
          {recentFiles.map((path) => {
            const parts = path.replace(/\\/g, "/").split("/");
            const name = parts.pop() ?? path;
            const dir  = parts.join("/").slice(-48) || "";
            return (
              <button
                key={path}
                onClick={() => onOpen(path)}
                title={path}
                className="w-full text-left px-3 py-1.5 rounded text-xs
                           hover:bg-surface-3 transition-colors
                           focus:outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
              >
                <span className="text-neutral-300">{name}</span>
                {dir && (
                  <span className="ml-2 text-neutral-700 text-[10px]">{dir}</span>
                )}
              </button>
            );
          })}
        </div>
      )}

      <p className="text-[11px] text-neutral-700 mt-2">
        Ctrl+O to open · Ctrl+S to save · Ctrl+, settings · Chrome, Firefox, Edge, Safari exports supported
      </p>
    </div>
  );
}
