/**
 * Right pane: selected item details + sanitize controls.
 *
 * Running a pass hands the proposed change set to the review surface
 * (`ReviewPane`), which takes over the list and detail columns until the
 * user applies or discards it.
 */

import { useState, useRef, useEffect } from "react";
import { RUN_PASS_EVENT } from "../components/paletteCommands";
import { open as shellOpen } from "@tauri-apps/plugin-shell";
import { save } from "@tauri-apps/plugin-dialog";
import { useDocuments } from "../state/documents";
import { useToast } from "../hooks/useToast";
import { useT } from "../i18n/I18nProvider";
import { ipc } from "../ipc";
import {
  DownloadIcon,
  PencilIcon,
  SlidersIcon,
  TrashIcon,
} from "../components/Icons";
import { RuleSetEditorModal } from "../components/RuleSetEditorModal";
import { useRuleSetList, BUILTIN_DESCRIPTIONS } from "../hooks/useRuleSetList";
import type { FolderItem } from "../ipc/types";

type PassScope = "document" | "folder";

// ---------------------------------------------------------------------------

export default function DetailPane() {
  const t = useT();
  const { toast } = useToast();
  const { activeTab, tabs, selectedItem, pendingDelete, setPendingDelete,
          refreshTree, refreshList, folderStack, openReview } = useDocuments();
  const activeInfo = tabs.find((tab) => tab.id === activeTab) ?? null;

  const { summaries, refresh: refreshRuleSets } = useRuleSetList();

  const [ruleSetName, setRuleSetName] = useState<string>("Aggressive scrub");
  const [scope, setScope] = useState<PassScope>("document");
  const [running, setRunning] = useState(false);
  const [editorOpen, setEditorOpen] = useState(false);

  // "This folder" only means something below the root.
  const currentFolder = folderStack.at(-1) ?? null;
  const effectiveScope: PassScope = currentFolder ? scope : "document";

  // Keep ruleSetName valid when summaries update (e.g. user deletes active set).
  useEffect(() => {
    if (summaries.length === 0) return;
    if (!summaries.find((s) => s.name === ruleSetName)) {
      setRuleSetName(summaries[0].name);
    }
  }, [summaries, ruleSetName]);

  const selectedSummary = summaries.find((s) => s.name === ruleSetName);
  const description =
    BUILTIN_DESCRIPTIONS[ruleSetName] ??
    (selectedSummary
      ? `${selectedSummary.treatment_count} treatment${selectedSummary.treatment_count !== 1 ? "s" : ""}`
      : null);

  const handleRunPass = async () => {
    if (!activeTab) return;
    const folder = effectiveScope === "folder" ? currentFolder : null;
    const scopeLabel = folder
      ? t("review.scope.folder", { name: folder.name })
      : t("review.scope.document");
    setRunning(true);
    try {
      const preview = await ipc.runPass(activeTab, ruleSetName, folder?.id ?? null);
      if (preview.changes.length === 0) {
        toast(t("review.nothing", { scope: scopeLabel, ruleSet: ruleSetName }), "info");
      } else {
        openReview({ tabId: activeTab, preview, scopeLabel });
      }
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setRunning(false);
    }
  };

  const handleRunPassRef = useRef(handleRunPass);
  handleRunPassRef.current = handleRunPass;

  useEffect(() => {
    const onRunPass = () => {
      void handleRunPassRef.current();
    };
    window.addEventListener(RUN_PASS_EVENT, onRunPass);
    return () => window.removeEventListener(RUN_PASS_EVENT, onRunPass);
  }, []);

  return (
    <div className="flex flex-col h-full relative overflow-hidden">
      {/* ── Selected item info ───────────────────────────────────────────── */}
      {selectedItem ? (
        <ItemDetail
          item={selectedItem}
          activeTab={activeTab}
          pendingDelete={pendingDelete}
          onRequestDelete={() => setPendingDelete(selectedItem)}
          onCancelDelete={() => setPendingDelete(null)}
          onConfirmDelete={async () => {
            if (!activeTab || !pendingDelete) return;
            try {
              await ipc.deleteNode(activeTab, pendingDelete.id);
              setPendingDelete(null);
              await Promise.all([refreshTree(), refreshList()]);
            } catch {
              setPendingDelete(null);
            }
          }}
          onRename={async (newName: string) => {
            if (!activeTab) return;
            await ipc.renameNode(activeTab, selectedItem.id, newName);
            await Promise.all([refreshTree(), refreshList()]);
          }}
        />
      ) : activeInfo ? (
        <section className="p-3 border-b border-neutral-800 shrink-0 space-y-2">
          <span className="text-[10px] uppercase tracking-wider text-neutral-600 font-semibold">
            {t("detail.document")}
          </span>
          <p className="text-sm text-neutral-100 font-medium break-words">{activeInfo.title}</p>
          {activeInfo.path && (
            <p className="text-[11px] text-neutral-500 break-all" title={activeInfo.path}>
              {activeInfo.path}
            </p>
          )}
          <dl className="grid grid-cols-3 gap-2 pt-1">
            {(
              [
                ["detail.stat.bookmarks", activeInfo.stats.bookmark_count],
                ["detail.stat.folders", activeInfo.stats.folder_count],
                ["detail.stat.separators", activeInfo.stats.separator_count],
              ] as const
            ).map(([key, n]) => (
              <div key={key} className="rounded-md bg-surface-2 px-2 py-1.5">
                <dd className="text-sm text-neutral-100 tabular-nums">{n.toLocaleString()}</dd>
                <dt className="text-[10px] text-neutral-500">{t(key)}</dt>
              </div>
            ))}
          </dl>
          <p className="text-[11px] text-neutral-500 leading-snug">
            {activeInfo.dirty ? t("detail.edited") : t("detail.selectHint")}
          </p>
        </section>
      ) : null}

      {/* ── Sanitize section ─────────────────────────────────────────────── */}
      <section className="p-3 border-b border-neutral-800 shrink-0">
        <h2 className="text-xs font-semibold text-neutral-400 uppercase tracking-wider mb-2">
          Sanitize
        </h2>

        {/* Rule set picker */}
        <div className="mb-2">
          <div className="flex items-center justify-between mb-1">
            <label className="text-[10px] text-neutral-600 uppercase tracking-wider">
              Rule set
            </label>
            <button
              onClick={() => setEditorOpen(true)}
              title="Manage rule sets"
              className="flex items-center gap-1 text-[10px] text-neutral-600
                         hover:text-neutral-300 transition-colors
                         focus:outline-none focus-visible:ring-1
                         focus-visible:ring-accent rounded"
            >
              <SlidersIcon className="w-3 h-3" />
              Manage
            </button>
          </div>
          <select
            value={ruleSetName}
            onChange={(e) => setRuleSetName(e.target.value)}
            disabled={running || !activeTab}
            className="w-full bg-surface-2 border border-neutral-700 rounded px-2 py-1
                       text-xs text-neutral-200 focus:outline-none focus:ring-1
                       focus:ring-accent disabled:opacity-40 cursor-pointer"
          >
            {summaries.map((rs) => (
              <option key={rs.name} value={rs.name}>
                {rs.name}
              </option>
            ))}
          </select>
          {description && (
            <p className="mt-1 text-[10px] text-neutral-600 leading-snug">
              {description}
            </p>
          )}
        </div>

        {/* Scope: whole document, or the folder currently shown in the list */}
        <div className="mb-2">
          <span className="block text-[10px] text-neutral-600 uppercase tracking-wider mb-1">
            {t("detail.scope")}
          </span>
          <div role="radiogroup" aria-label={t("detail.scope")} className="grid grid-cols-2 gap-1">
            {(["document", "folder"] as const).map((value) => {
              const disabled = value === "folder" && !currentFolder;
              const checked = effectiveScope === value;
              return (
                <button
                  key={value}
                  type="button"
                  role="radio"
                  aria-checked={checked}
                  disabled={disabled || running}
                  onClick={() => setScope(value)}
                  title={value === "folder" && currentFolder ? currentFolder.name : undefined}
                  className={`px-2 py-1 rounded border text-[11px] truncate transition-colors
                              disabled:opacity-40 ${
                                checked
                                  ? "border-accent/60 bg-accent/10 text-accent"
                                  : "border-neutral-800 text-neutral-400 hover:text-neutral-200"
                              }`}
                >
                  {value === "document"
                    ? t("detail.scope.document")
                    : currentFolder
                      ? currentFolder.name
                      : t("detail.scope.folder")}
                </button>
              );
            })}
          </div>
        </div>

        <button
          onClick={handleRunPass}
          disabled={running || !activeTab}
          className="w-full py-1.5 rounded bg-accent hover:bg-accent-hover
                     disabled:opacity-40 text-on-accent font-medium text-xs
                     transition-colors focus:outline-none focus-visible:ring-2
                     focus-visible:ring-accent"
        >
          {running ? "Analysing…" : "Run pass"}
        </button>
      </section>

      {/* ── Rule-set editor modal ────────────────────────────────────────── */}
      <RuleSetEditorModal
        open={editorOpen}
        initialSet={ruleSetName}
        onClose={() => setEditorOpen(false)}
        onRefreshList={refreshRuleSets}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Selected item detail card
// ---------------------------------------------------------------------------

function ItemDetail({
  item,
  activeTab,
  pendingDelete,
  onRequestDelete,
  onCancelDelete,
  onConfirmDelete,
  onRename,
}: {
  item: FolderItem;
  activeTab: number | null;
  pendingDelete: FolderItem | null;
  onRequestDelete: () => void;
  onCancelDelete: () => void;
  onConfirmDelete: () => Promise<void>;
  onRename: (newName: string) => Promise<void>;
}) {
  const [copied, setCopied] = useState(false);
  const [isRenaming, setIsRenaming] = useState(false);
  const [renameValue, setRenameValue] = useState("");
  const [deleting, setDeleting] = useState(false);
  const [exportingFolder, setExportingFolder] = useState(false);
  const renameRef = useRef<HTMLInputElement>(null);

  // Reset rename state when the selected item changes.
  useEffect(() => {
    setIsRenaming(false);
  }, [item.id]);

  // Focus the rename input when it appears.
  useEffect(() => {
    if (isRenaming) renameRef.current?.select();
  }, [isRenaming]);

  const startRename = () => {
    setRenameValue(item.title || "");
    setIsRenaming(true);
  };

  const commitRename = async () => {
    const trimmed = renameValue.trim();
    if (trimmed && trimmed !== item.title) {
      try { await onRename(trimmed); } catch { /* ignore */ }
    }
    setIsRenaming(false);
  };

  const cancelRename = () => setIsRenaming(false);

  const handleDelete = async () => {
    setDeleting(true);
    try { await onConfirmDelete(); } finally { setDeleting(false); }
  };

  const copyUrl = async () => {
    if (!item.url) return;
    try {
      await navigator.clipboard.writeText(item.url);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch { /* clipboard unavailable */ }
  };

  const openUrl = async () => {
    if (!item.url) return;
    try { await shellOpen(item.url); } catch { /* ignore */ }
  };

  const handleExportFolder = async () => {
    if (activeTab === null || item.kind !== "folder") return;
    setExportingFolder(true);
    try {
      const fname = (item.title || "folder").replace(/[\\/:*?"<>|]/g, "_");
      const path = await save({
        filters: [{ name: "HTML bookmark file", extensions: ["html", "htm"] }],
        defaultPath: `${fname}.html`,
      });
      if (typeof path === "string") {
        await ipc.export(activeTab, { kind: "subtree", root_id: item.id }, path);
      }
    } catch {
      // user cancelled or export failed; silent for now
    } finally {
      setExportingFolder(false);
    }
  };

  const kindLabel =
    item.kind === "folder" ? "Folder" : item.kind === "separator" ? "Separator" : "Bookmark";
  const isConfirmingDelete = pendingDelete?.id === item.id;
  const canRename = item.kind !== "separator";

  return (
    <section className="p-3 border-b border-neutral-800 shrink-0 space-y-2">
      {/* Kind badge + title row */}
      <div>
        <div className="flex items-center justify-between mb-0.5">
          <span className="text-[10px] uppercase tracking-wider text-neutral-600 font-semibold">
            {kindLabel}
          </span>
          {/* Rename + export-folder + delete action buttons */}
          {activeTab !== null && (
            <div className="flex items-center gap-1">
              {canRename && !isRenaming && (
                <button
                  onClick={startRename}
                  title="Rename (F2)"
                  className="p-0.5 text-neutral-600 hover:text-neutral-300 transition-colors
                             focus:outline-none"
                >
                  <PencilIcon className="w-3 h-3" />
                </button>
              )}
              {item.kind === "folder" && !isConfirmingDelete && (
                <button
                  onClick={handleExportFolder}
                  disabled={exportingFolder}
                  title="Export this folder as a standalone bookmark file"
                  className="p-0.5 text-neutral-600 hover:text-neutral-300 transition-colors
                             disabled:opacity-40 focus:outline-none"
                >
                  <DownloadIcon className="w-3 h-3" />
                </button>
              )}
              {!isConfirmingDelete && (
                <button
                  onClick={onRequestDelete}
                  title="Delete"
                  className="p-0.5 text-neutral-600 hover:text-danger transition-colors
                             focus:outline-none"
                >
                  <TrashIcon className="w-3 h-3" />
                </button>
              )}
            </div>
          )}
        </div>

        {/* Inline rename input or title display */}
        {isRenaming ? (
          <input
            ref={renameRef}
            value={renameValue}
            onChange={(e) => setRenameValue(e.target.value)}
            onBlur={commitRename}
            onKeyDown={(e) => {
              if (e.key === "Enter") { e.preventDefault(); commitRename(); }
              if (e.key === "Escape") { e.preventDefault(); cancelRename(); }
            }}
            className="w-full bg-surface-2 border border-accent rounded px-2 py-0.5
                       text-xs text-neutral-100 focus:outline-none"
          />
        ) : (
          <p className="text-xs text-neutral-200 font-medium leading-snug break-words">
            {item.title || <em className="text-neutral-600">untitled</em>}
          </p>
        )}
      </div>

      {/* Delete confirmation strip */}
      {isConfirmingDelete && (
        <div className="rounded border border-danger/30 bg-danger/10 px-2 py-1.5 space-y-1.5">
          <p className="text-[10px] text-danger leading-snug">
            Delete <span className="font-semibold">"{item.title || "this item"}"</span>?
            {item.kind === "folder" && " All contents will be removed."}
            {" "}This cannot be undone.
          </p>
          <div className="flex gap-2">
            <button
              onClick={handleDelete}
              disabled={deleting}
              className="flex-1 py-0.5 rounded bg-danger hover:bg-danger/80 text-on-danger
                         text-[10px] font-medium transition-colors disabled:opacity-40
                         focus:outline-none"
            >
              {deleting ? "Deleting…" : "Delete"}
            </button>
            <button
              onClick={onCancelDelete}
              className="flex-1 py-0.5 rounded bg-surface-3 hover:bg-surface-4
                         text-neutral-400 text-[10px] transition-colors focus:outline-none"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* URL */}
      {item.url && (
        <div>
          <span className="text-[10px] uppercase tracking-wider text-neutral-600">URL</span>
          <p className="mt-0.5 text-[11px] text-neutral-400 break-all leading-snug line-clamp-3">
            {item.url}
          </p>
          <div className="flex items-center gap-2 mt-1">
            <button
              onClick={copyUrl}
              className="text-[10px] text-neutral-500 hover:text-neutral-200 transition-colors"
            >
              {copied ? "✓ Copied" : "Copy URL"}
            </button>
            <span className="text-neutral-700">·</span>
            <button
              onClick={openUrl}
              className="text-[10px] text-neutral-500 hover:text-neutral-200 transition-colors"
            >
              Open in browser ↗
            </button>
          </div>
        </div>
      )}

      {/* Dates */}
      {(item.add_date || item.last_modified) && (
        <div className="text-[10px] text-neutral-600 space-y-0.5">
          {item.add_date && <div>Added: {formatDate(item.add_date)}</div>}
          {item.last_modified && <div>Modified: {formatDate(item.last_modified)}</div>}
        </div>
      )}
    </section>
  );
}

function formatDate(unix: number): string {
  return new Date(unix * 1000).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

