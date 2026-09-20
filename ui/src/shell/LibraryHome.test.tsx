/**
 * Library home: empty vs populated collection, recovery banner, one-click open.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import LibraryHome from "./LibraryHome";
import { I18nProvider } from "../i18n/I18nProvider";
import { ipc } from "../ipc";
import type { TabInfo } from "../ipc/types";

const store = {
  tabs: [] as TabInfo[],
  refreshTabs: vi.fn().mockResolvedValue(undefined),
  setActiveTab: vi.fn().mockResolvedValue(undefined),
};

vi.mock("../ipc", () => ({
  ipc: {
    listRecentFiles: vi.fn(),
    getRecoveryState: vi.fn(),
    clearRecentFiles: vi.fn(),
    restoreRecoverySession: vi.fn(),
    dismissRecoverySession: vi.fn(),
  },
}));

vi.mock("../state/documents", () => {
  const useDocuments = Object.assign(() => store, {
    getState: () => store,
  });
  return { useDocuments };
});

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn().mockResolvedValue(null),
}));

const RECENTS = [
  "C:/lantern/test/fixtures/small.html",
  "C:/Users/fixture/Documents/bookmarks-firefox.html",
  "C:/Users/fixture/Downloads/chrome-bookmarks.html",
];

function renderHome(onOpen = vi.fn().mockResolvedValue(undefined)) {
  return render(
    <I18nProvider locale="en">
      <LibraryHome onOpen={onOpen} />
    </I18nProvider>,
  );
}

beforeEach(() => {
  store.tabs = [];
  store.refreshTabs.mockClear();
  store.setActiveTab.mockClear();
  vi.mocked(ipc.listRecentFiles).mockReset();
  vi.mocked(ipc.getRecoveryState).mockReset();
  vi.mocked(ipc.clearRecentFiles).mockReset();
  vi.mocked(ipc.restoreRecoverySession).mockReset();
  vi.mocked(ipc.dismissRecoverySession).mockReset();
  vi.mocked(ipc.getRecoveryState).mockResolvedValue({ paths: [] });
});

describe("LibraryHome", () => {
  it("renders the empty-library invitation when there are no recents", async () => {
    vi.mocked(ipc.listRecentFiles).mockResolvedValue([]);
    renderHome();

    expect(
      await screen.findByRole("heading", { name: "Your library is empty" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Open a bookmark file to start a private collection/),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open file…" })).toBeInTheDocument();
    expect(screen.queryByText("Recent volumes")).not.toBeInTheDocument();
    expect(screen.queryByText("small.html")).not.toBeInTheDocument();
  });

  it("renders a collection of recent volumes when the library is populated", async () => {
    vi.mocked(ipc.listRecentFiles).mockResolvedValue(RECENTS);
    renderHome();

    expect(
      await screen.findByRole("heading", { name: "Library" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Recent volumes")).toBeInTheDocument();
    expect(screen.getByText("3 volumes")).toBeInTheDocument();
    expect(screen.getByText("small.html")).toBeInTheDocument();
    expect(screen.getByText("C:/lantern/test/fixtures")).toBeInTheDocument();
    expect(screen.getByText("bookmarks-firefox.html")).toBeInTheDocument();
    expect(screen.getByText("C:/Users/fixture/Documents")).toBeInTheDocument();
    expect(screen.getByText("chrome-bookmarks.html")).toBeInTheDocument();
    expect(screen.getByText("Most recent")).toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: "Your library is empty" }),
    ).not.toBeInTheDocument();
  });

  it("opens a recent volume in one click", async () => {
    const user = userEvent.setup();
    const onOpen = vi.fn().mockResolvedValue(undefined);
    vi.mocked(ipc.listRecentFiles).mockResolvedValue(RECENTS);
    renderHome(onOpen);

    await screen.findByText("bookmarks-firefox.html");
    await user.click(
      screen.getByRole("button", { name: "Open bookmarks-firefox.html" }),
    );

    expect(onOpen).toHaveBeenCalledTimes(1);
    expect(onOpen).toHaveBeenCalledWith(
      "C:/Users/fixture/Documents/bookmarks-firefox.html",
    );
  });

  it("focuses an already-open tab instead of opening a duplicate", async () => {
    const user = userEvent.setup();
    const onOpen = vi.fn().mockResolvedValue(undefined);
    store.tabs = [
      {
        id: 7,
        title: "small.html",
        path: "C:/lantern/test/fixtures/small.html",
        dirty: false,
        stats: { bookmark_count: 1, folder_count: 1, separator_count: 0 },
      },
    ];
    vi.mocked(ipc.listRecentFiles).mockResolvedValue(RECENTS);
    renderHome(onOpen);

    await screen.findByText("small.html");
    await user.click(screen.getByRole("button", { name: "Open small.html" }));

    expect(store.setActiveTab).toHaveBeenCalledWith(7);
    expect(onOpen).not.toHaveBeenCalled();
  });

  it("keeps the crash-recovery banner on both empty and populated homes", async () => {
    vi.mocked(ipc.listRecentFiles).mockResolvedValue([]);
    vi.mocked(ipc.getRecoveryState).mockResolvedValue({
      paths: ["C:/tmp/recovered.html"],
    });
    renderHome();

    expect(
      await screen.findByText("Recover previous session"),
    ).toBeInTheDocument();
    expect(screen.getByText("C:/tmp/recovered.html")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Restore" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Dismiss" })).toBeInTheDocument();
  });

  it("clears the recent-volume list from the populated home", async () => {
    const user = userEvent.setup();
    vi.mocked(ipc.listRecentFiles).mockResolvedValue(RECENTS);
    vi.mocked(ipc.clearRecentFiles).mockResolvedValue(undefined);
    renderHome();

    await screen.findByText("Recent volumes");
    await user.click(screen.getByRole("button", { name: "Clear" }));

    await waitFor(() => {
      expect(ipc.clearRecentFiles).toHaveBeenCalled();
    });
    expect(
      await screen.findByRole("heading", { name: "Your library is empty" }),
    ).toBeInTheDocument();
  });
});
