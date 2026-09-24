import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import type { ReactNode } from "react";

import { I18nProvider } from "../i18n/I18nProvider";
import { isOpenable, suggestedCopyPath, useFileActions } from "./fileActions";

const toast = vi.fn();
const store = {
  tabs: [] as { id: number; title: string; path: string | null }[],
  activeTab: null as number | null,
  openFile: vi.fn(),
  setActiveTab: vi.fn(),
  refreshTabs: vi.fn(),
  refreshTree: vi.fn(),
  refreshList: vi.fn(),
};

vi.mock("./documents", () => {
  const useDocuments = Object.assign(() => store, { getState: () => store });
  return { useDocuments };
});
vi.mock("../hooks/useToast", () => ({ useToast: () => ({ toast }) }));
vi.mock("../ipc", () => ({ ipc: { export: vi.fn(), undo: vi.fn(), redo: vi.fn() } }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));

import { ipc } from "../ipc";
import { save } from "@tauri-apps/plugin-dialog";

const wrapper = ({ children }: { children: ReactNode }) => <I18nProvider locale="en">{children}</I18nProvider>;
/** Render the hook, then run one action inside act(). */
async function run(fn: (a: ReturnType<typeof useFileActions>) => Promise<unknown>) {
  const { result } = renderHook(() => useFileActions(), { wrapper });
  await act(async () => {
    await fn(result.current);
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  store.tabs = [{ id: 1, title: "bookmarks.html", path: "C:/data/bookmarks.html" }];
  store.activeTab = 1;
});

describe("helpers", () => {
  it("accepts HTML exports and Chrome's extensionless Bookmarks file", () => {
    expect(isOpenable("C:/x/export.HTML")).toBe(true);
    expect(isOpenable("/home/me/.config/chromium/Default/Bookmarks")).toBe(true);
    expect(isOpenable("C:/x/photo.png")).toBe(false);
  });

  it("suggests a .clean.html sibling of the original", () => {
    expect(suggestedCopyPath("C:\\data\\bookmarks.html", "bookmarks.html")).toBe("C:\\data\\bookmarks.clean.html");
    expect(suggestedCopyPath(null, "Merged")).toBe("Merged.clean.html");
  });
});

describe("openPaths", () => {
  it("focuses an already-open tab instead of opening it twice, and rejects unsupported files", async () => {
    store.activeTab = null;
    await run((a) => a.openPaths(["c:/DATA/bookmarks.html", "C:/x/notes.txt"]));
    expect(store.openFile).not.toHaveBeenCalled();
    expect(store.setActiveTab).toHaveBeenCalledWith(1);
    expect(toast).toHaveBeenCalledWith(expect.stringContaining("notes.txt"), "error");
  });
});

describe("saveCopy", () => {
  it("asks for a destination once, then reuses it", async () => {
    vi.mocked(save).mockResolvedValue("C:/data/clean.html");
    await run((a) => a.saveCopy());
    await run((a) => a.saveCopy());
    expect(save).toHaveBeenCalledTimes(1);
    expect(ipc.export).toHaveBeenCalledTimes(2);
    expect(ipc.export).toHaveBeenLastCalledWith(1, { kind: "whole_document" }, "C:/data/clean.html");
    expect(toast).toHaveBeenLastCalledWith("Saved a copy to clean.html.", "success");
  });

  it("refuses to overwrite the file the document was opened from", async () => {
    store.tabs = [{ id: 2, title: "b.html", path: "C:/data/b.html" }];
    store.activeTab = 2;
    vi.mocked(save).mockResolvedValue("c:\\DATA\\b.html");
    await run((a) => a.saveCopy({ saveAs: true }));
    expect(ipc.export).not.toHaveBeenCalled();
    expect(toast).toHaveBeenCalledWith(expect.stringContaining("never overwrites"), "error");
  });

  it("does nothing when the dialog is cancelled", async () => {
    store.tabs = [{ id: 3, title: "c.html", path: "C:/data/c.html" }];
    store.activeTab = 3;
    vi.mocked(save).mockResolvedValue(null);
    await run((a) => a.saveCopy());
    expect(ipc.export).not.toHaveBeenCalled();
    expect(toast).not.toHaveBeenCalled();
  });
});

describe("undo / redo", () => {
  it("says so when there is nothing to undo", async () => {
    vi.mocked(ipc.undo).mockRejectedValue(new Error("nothing"));
    await run((a) => a.undo());
    expect(toast).toHaveBeenCalledWith("Nothing to undo.", "info");
  });
});
