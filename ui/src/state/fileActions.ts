/**
 * File-level actions shared by the title bar, keyboard shortcuts, command
 * palette, library home and drag-and-drop: open, save a copy, undo / redo.
 * Every action reports its outcome through a toast.
 *
 * Lantern never writes the file a document was opened from (see README):
 * "Save" writes a *copy*.  The first save of a tab asks where; later saves
 * reuse that copy's path.  Picking the original file as the destination is
 * refused.
 */

import { useCallback } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";

import { ipc } from "../ipc";
import { useDocuments } from "./documents";
import { useToast } from "../hooks/useToast";
import { useT } from "../i18n/I18nProvider";
import type { TabId } from "../ipc/types";

/** Extensions Lantern can open (Netscape HTML exports and Chrome JSON). */
export const OPENABLE_EXTENSIONS = ["html", "htm", "json"];

const OPEN_FILTERS = [
  { name: "Bookmark files", extensions: OPENABLE_EXTENSIONS },
  { name: "All files", extensions: ["*"] },
];
const SAVE_FILTERS = [{ name: "HTML bookmark file", extensions: ["html", "htm"] }];

/** Where each tab's copy was last saved this session. */
const savedCopyPath = new Map<TabId, string>();

export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export function isOpenable(path: string): boolean {
  const ext = fileName(path).split(".").pop()?.toLowerCase() ?? "";
  // Chrome's profile file is literally named "Bookmarks" (no extension).
  return OPENABLE_EXTENSIONS.includes(ext) || fileName(path) === "Bookmarks";
}

/** `<dir>/<stem>.clean.html` next to the original, as the save dialog's suggestion. */
export function suggestedCopyPath(original: string | null, title: string): string {
  if (!original) return `${title || "bookmarks"}.clean.html`;
  const name = fileName(original);
  const dir = original.slice(0, original.length - name.length);
  const stem = name.replace(/\.(html?|json)$/i, "");
  return `${dir}${stem}.clean.html`;
}

const samePath = (a: string, b: string) =>
  a.replace(/\\/g, "/").toLowerCase() === b.replace(/\\/g, "/").toLowerCase();

export function useFileActions() {
  const t = useT();
  const { toast } = useToast();
  const { openFile, setActiveTab, refreshTabs, refreshTree, refreshList } = useDocuments();

  const openPaths = useCallback(
    async (paths: string[]) => {
      let lastOpened: TabId | null = null;
      for (const path of paths) {
        if (!isOpenable(path)) {
          toast(t("file.unsupported", { name: fileName(path) }), "error");
          continue;
        }
        const existing = useDocuments.getState().tabs.find((tab) => tab.path && samePath(tab.path, path));
        if (existing) {
          lastOpened = existing.id;
          continue;
        }
        try {
          await openFile(path);
          lastOpened = useDocuments.getState().activeTab;
        } catch (e) {
          toast(t("file.openFailed", { name: fileName(path), reason: String(e) }), "error");
        }
      }
      if (lastOpened !== null && useDocuments.getState().activeTab !== lastOpened) {
        await setActiveTab(lastOpened);
      }
    },
    [openFile, setActiveTab, toast, t],
  );

  const pickAndOpen = useCallback(async () => {
    const selected = await open({ filters: OPEN_FILTERS, multiple: true });
    if (selected === null) return;
    await openPaths(Array.isArray(selected) ? selected : [selected]);
  }, [openPaths]);

  const writeCopy = useCallback(
    async (tabId: TabId, forceDialog: boolean) => {
      const tab = useDocuments.getState().tabs.find((x) => x.id === tabId);
      if (!tab) return;
      let dest = forceDialog ? undefined : savedCopyPath.get(tabId);
      if (!dest) {
        const picked = await save({
          filters: SAVE_FILTERS,
          defaultPath: savedCopyPath.get(tabId) ?? suggestedCopyPath(tab.path, tab.title),
        });
        if (typeof picked !== "string") return; // cancelled
        dest = picked;
      }
      if (tab.path && samePath(dest, tab.path)) {
        toast(t("file.refuseOriginal"), "error");
        return;
      }
      try {
        await ipc.export(tabId, { kind: "whole_document" }, dest);
        savedCopyPath.set(tabId, dest);
        toast(t("file.saved", { name: fileName(dest) }), "success");
      } catch (e) {
        toast(t("file.saveFailed", { reason: String(e) }), "error");
      }
    },
    [toast, t],
  );

  const saveCopy = useCallback(
    async (opts: { saveAs?: boolean } = {}) => {
      const { activeTab } = useDocuments.getState();
      if (activeTab !== null) await writeCopy(activeTab, opts.saveAs ?? false);
    },
    [writeCopy],
  );

  const history = useCallback(
    async (direction: "undo" | "redo") => {
      const { activeTab } = useDocuments.getState();
      if (activeTab === null) return;
      try {
        await (direction === "undo" ? ipc.undo(activeTab) : ipc.redo(activeTab));
        await Promise.all([refreshTabs(), refreshTree(), refreshList()]);
      } catch {
        toast(t(direction === "undo" ? "file.nothingToUndo" : "file.nothingToRedo"), "info");
      }
    },
    [refreshTabs, refreshTree, refreshList, toast, t],
  );

  return {
    openPaths,
    pickAndOpen,
    saveCopy,
    undo: useCallback(() => history("undo"), [history]),
    redo: useCallback(() => history("redo"), [history]),
  };
}
