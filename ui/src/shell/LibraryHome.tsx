/**
 * Library home: the stacks.
 *
 * Shown when no document tab is focused.  This is the product surface
 * that frames Lantern as a private archive rather than a one-shot file
 * opener.  Crash-recovery still lives here; opening a recent volume is
 * one click.  The three-pane workspace is "inside a volume."
 *
 * Empty and populated libraries are different layouts: an empty library
 * is a centered invitation; a populated one is a collection of volumes.
 */

import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import { useDocuments } from "../state/documents";
import { ipc } from "../ipc";
import { useT } from "../i18n/I18nProvider";
import { FolderIcon } from "../components/Icons";
import { parseVolumePath, type VolumePresence } from "./volumePath";

export default function LibraryHome({
  onOpen,
}: {
  onOpen: (path: string) => Promise<void>;
}) {
  const t = useT();
  const { refreshTabs, setActiveTab } = useDocuments();
  const [recentFiles, setRecentFiles] = useState<string[]>([]);
  const [recoveryPaths, setRecoveryPaths] = useState<string[]>([]);
  const [recoveryMessage, setRecoveryMessage] = useState<string | null>(null);
  const [restoringSession, setRestoringSession] = useState(false);
  const [clearingRecent, setClearingRecent] = useState(false);

  const loadLibraryData = async () => {
    try {
      const [recent, recovery] = await Promise.all([
        ipc.listRecentFiles(),
        ipc.getRecoveryState(),
      ]);
      setRecentFiles(recent);
      setRecoveryPaths(recovery.paths);
    } catch {
      // not in Tauri context (or fixture IPC rejected): stay empty
    }
  };

  useEffect(() => {
    loadLibraryData();
  }, []);

  const handleOpen = async () => {
    const selected = await open({
      filters: [
        { name: "Bookmark files", extensions: ["html", "htm", "json"] },
        { name: "All files", extensions: ["*"] },
      ],
      multiple: false,
    });
    if (typeof selected === "string") {
      await onOpen(selected);
    }
  };

  const handleOpenVolume = async (path: string) => {
    const { tabs } = useDocuments.getState();
    const existing = tabs.find((tab) => tab.path === path);
    if (existing) {
      await setActiveTab(existing.id);
      return;
    }
    await onOpen(path);
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
          t("library.recovery.partial", {
            restored: report.restored_paths.length,
            failed: report.failed_paths.length,
          }),
        );
      }
      await loadLibraryData();
    } catch {
      setRecoveryMessage(t("library.recovery.failed"));
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

  const volumes = recentFiles.map(parseVolumePath);
  const populated = volumes.length > 0;

  return (
    <div
      role="region"
      aria-label={t("library.region")}
      className={`flex-1 min-h-0 overflow-y-auto ${
        populated
          ? "px-8 py-6"
          : "flex flex-col items-center justify-center gap-6 text-neutral-400"
      }`}
    >
      {populated ? (
        <PopulatedLibrary
          volumes={volumes}
          recoveryPaths={recoveryPaths}
          recoveryMessage={recoveryMessage}
          restoringSession={restoringSession}
          clearingRecent={clearingRecent}
          onOpen={handleOpen}
          onOpenVolume={handleOpenVolume}
          onRestore={handleRestoreSession}
          onDismissRecovery={handleDismissRecovery}
          onClearRecent={handleClearRecent}
        />
      ) : (
        <EmptyLibrary
          recoveryPaths={recoveryPaths}
          recoveryMessage={recoveryMessage}
          restoringSession={restoringSession}
          onOpen={handleOpen}
          onRestore={handleRestoreSession}
          onDismissRecovery={handleDismissRecovery}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Empty library: centered invitation
// ---------------------------------------------------------------------------

function EmptyLibrary({
  recoveryPaths,
  recoveryMessage,
  restoringSession,
  onOpen,
  onRestore,
  onDismissRecovery,
}: {
  recoveryPaths: string[];
  recoveryMessage: string | null;
  restoringSession: boolean;
  onOpen: () => Promise<void>;
  onRestore: () => Promise<void>;
  onDismissRecovery: () => Promise<void>;
}) {
  const t = useT();

  return (
    <>
      <LanternLogo />

      <div className="text-center max-w-md">
        <h1 className="text-xl font-semibold text-neutral-200 mb-1 tracking-wide">
          {t("library.empty.title")}
        </h1>
        <p className="text-sm text-neutral-500">
          {t("library.empty.description")}
        </p>
      </div>

      <OpenFileButton onClick={onOpen} />

      {recoveryPaths.length > 0 && (
        <RecoveryBanner
          paths={recoveryPaths}
          restoring={restoringSession}
          onRestore={onRestore}
          onDismiss={onDismissRecovery}
        />
      )}

      {recoveryMessage && (
        <p className="text-xs text-neutral-500">{recoveryMessage}</p>
      )}

      <p className="text-xs text-neutral-500">{t("library.paletteHint")}</p>
      <p className="text-[11px] text-neutral-700">{t("library.hint")}</p>
    </>
  );
}

// ---------------------------------------------------------------------------
// Populated library: collection of recent volumes
// ---------------------------------------------------------------------------

function PopulatedLibrary({
  volumes,
  recoveryPaths,
  recoveryMessage,
  restoringSession,
  clearingRecent,
  onOpen,
  onOpenVolume,
  onRestore,
  onDismissRecovery,
  onClearRecent,
}: {
  volumes: VolumePresence[];
  recoveryPaths: string[];
  recoveryMessage: string | null;
  restoringSession: boolean;
  clearingRecent: boolean;
  onOpen: () => Promise<void>;
  onOpenVolume: (path: string) => Promise<void>;
  onRestore: () => Promise<void>;
  onDismissRecovery: () => Promise<void>;
  onClearRecent: () => Promise<void>;
}) {
  const t = useT();

  return (
    <div className="mx-auto w-full max-w-4xl flex flex-col gap-6">
      <header className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-xl font-semibold text-neutral-100 tracking-wide">
            {t("library.title")}
          </h1>
          <p className="mt-1 text-sm text-neutral-500">{t("library.subtitle")}</p>
        </div>
        <OpenFileButton onClick={onOpen} />
      </header>

      {recoveryPaths.length > 0 && (
        <RecoveryBanner
          paths={recoveryPaths}
          restoring={restoringSession}
          onRestore={onRestore}
          onDismiss={onDismissRecovery}
          className=""
        />
      )}

      {recoveryMessage && (
        <p className="text-xs text-neutral-500">{recoveryMessage}</p>
      )}

      <section aria-labelledby="library-recent-heading">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-baseline gap-2">
            <h2
              id="library-recent-heading"
              className="text-[11px] font-semibold uppercase tracking-wider text-neutral-400"
            >
              {t("library.recentHeading")}
            </h2>
            <span className="text-[11px] text-neutral-600">
              {t("library.recentCount", { n: volumes.length })}
            </span>
          </div>
          <button
            type="button"
            onClick={onClearRecent}
            disabled={clearingRecent}
            className="text-[11px] text-neutral-500 hover:text-neutral-200 transition-colors
                       disabled:opacity-40 focus:outline-none focus-visible:ring-1
                       focus-visible:ring-accent rounded px-1"
          >
            {clearingRecent ? t("library.clearing") : t("library.clearRecent")}
          </button>
        </div>

        <ul className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          {volumes.map((volume, index) => (
            <li key={volume.path}>
              <VolumeCard
                volume={volume}
                mostRecent={index === 0}
                onOpen={() => onOpenVolume(volume.path)}
              />
            </li>
          ))}
        </ul>
      </section>

      <footer className="flex flex-col gap-1">
        <p className="text-xs text-neutral-500">{t("library.paletteHint")}</p>
        <p className="text-[11px] text-neutral-700">{t("library.hint")}</p>
      </footer>
    </div>
  );
}

function VolumeCard({
  volume,
  mostRecent,
  onOpen,
}: {
  volume: VolumePresence;
  mostRecent: boolean;
  onOpen: () => void;
}) {
  const t = useT();
  const lastOpened = volume.lastOpened;

  return (
    <button
      type="button"
      onClick={onOpen}
      title={volume.path}
      aria-label={t("library.openVolume", { name: volume.name })}
      className="w-full text-left rounded-lg border border-neutral-800 bg-surface-2/70
                 px-4 py-3 hover:border-accent/40 hover:bg-surface-3 transition-colors
                 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent/70"
    >
      <div className="flex items-start gap-3">
        <span className="mt-0.5 text-accent shrink-0" aria-hidden>
          <FolderIcon className="w-4 h-4" />
        </span>
        <span className="min-w-0 flex-1">
          <span className="flex items-center gap-2">
            <span className="text-sm font-medium text-neutral-100 truncate">
              {volume.name}
            </span>
            {mostRecent && (
              <span className="shrink-0 text-[10px] uppercase tracking-wider
                               text-accent/90 border border-accent/25 rounded px-1.5 py-0.5">
                {t("library.mostRecent")}
              </span>
            )}
          </span>
          {volume.directory && (
            <span className="mt-0.5 block text-[11px] text-neutral-500 truncate">
              {volume.directory}
            </span>
          )}
          {lastOpened && (
            <span className="mt-1 block text-[11px] text-neutral-600">
              {lastOpened}
            </span>
          )}
        </span>
      </div>
    </button>
  );
}

function OpenFileButton({ onClick }: { onClick: () => void }) {
  const t = useT();
  return (
    <button
      type="button"
      onClick={onClick}
      className="px-5 py-2 rounded-md bg-accent hover:bg-accent-hover text-neutral-950
                 font-semibold text-sm transition-colors focus:outline-none
                 focus-visible:ring-2 focus-visible:ring-accent/70 shrink-0"
    >
      {t("library.open")}
    </button>
  );
}

function RecoveryBanner({
  paths,
  restoring,
  onRestore,
  onDismiss,
  className,
}: {
  paths: string[];
  restoring: boolean;
  onRestore: () => void;
  onDismiss: () => void;
  className?: string;
}) {
  const t = useT();

  return (
    <div className={`w-full rounded-lg border border-accent/20 bg-surface-2/80 px-4 py-3 ${className ?? "max-w-lg"}`}>
      <div className="flex items-center justify-between gap-3">
        <div>
          <p className="text-xs font-semibold uppercase tracking-wider text-accent/80">
            {t("library.recovery.title")}
          </p>
          <p className="mt-1 text-xs text-neutral-500">
            {t("library.recovery.description")}
          </p>
        </div>
        <div className="flex gap-2 shrink-0">
          <button
            type="button"
            onClick={onDismiss}
            disabled={restoring}
            className="px-3 py-1.5 rounded border border-neutral-700 text-xs text-neutral-400
                       hover:text-neutral-200 hover:border-neutral-500 transition-colors
                       disabled:opacity-40"
          >
            {t("library.recovery.dismiss")}
          </button>
          <button
            type="button"
            onClick={onRestore}
            disabled={restoring}
            className="px-3 py-1.5 rounded bg-accent text-neutral-950 text-xs font-semibold
                       hover:bg-accent-hover transition-colors disabled:opacity-40"
          >
            {restoring ? t("library.recovery.restoring") : t("library.recovery.restore")}
          </button>
        </div>
      </div>
      <div className="mt-3 space-y-1.5">
        {paths.slice(0, 5).map((path) => (
          <div key={path} className="text-[11px] text-neutral-500 break-all">
            {path}
          </div>
        ))}
        {paths.length > 5 && (
          <div className="text-[11px] text-neutral-600">
            {t("library.recovery.more", { n: paths.length - 5 })}
          </div>
        )}
      </div>
    </div>
  );
}

/**
 * Full-size lantern SVG for the empty-library invitation.
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
      <rect x="5" y="7" width="18" height="22" rx="3" opacity="0.18" />
      <rect x="5" y="7" width="18" height="22" rx="3"
            fill="none" stroke="currentColor" strokeWidth="1.8" />
      <line x1="5" y1="18" x2="23" y2="18"
            stroke="currentColor" strokeWidth="1.1" opacity="0.45" />
      <line x1="14" y1="7" x2="14" y2="29"
            stroke="currentColor" strokeWidth="1.1" opacity="0.45" />
      <rect x="9" y="4" width="10" height="4" rx="1.5" />
      <path d="M10 4 Q14 0.5 18 4"
            fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
      <ellipse cx="14" cy="18" rx="3.5" ry="4.5" opacity="0.65" />
    </svg>
  );
}
