import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { MergePickerModal } from "./MergePickerModal";
import { ipc } from "../ipc";
import type { TabInfo, TreeView } from "../ipc/types";

// Stub the IPC layer: the modal calls `getTree` lazily on expand and
// `mergeDocuments` on the final merge action.
vi.mock("../ipc", () => ({
  ipc: {
    getTree: vi.fn(),
    mergeDocuments: vi.fn(),
  },
}));

const tab1: TabInfo = {
  id: 1,
  title: "Bookmarks one",
  path: "/tmp/one.html",
  dirty: false,
  stats: { bookmark_count: 0, folder_count: 0, separator_count: 0 },
};

const tab2: TabInfo = {
  id: 2,
  title: "Bookmarks two",
  path: "/tmp/two.html",
  dirty: false,
  stats: { bookmark_count: 0, folder_count: 0, separator_count: 0 },
};

const tree1: TreeView = {
  root: {
    id: 0,
    name: "Bookmarks",
    children: [
      { id: 11, name: "Folder A", children: [] },
      { id: 12, name: "Folder B", children: [] },
    ],
  },
};

beforeEach(() => {
  vi.mocked(ipc.getTree).mockReset();
  vi.mocked(ipc.mergeDocuments).mockReset();
});

describe("MergePickerModal", () => {
  it("renders the open tabs in the picker", () => {
    render(
      <MergePickerModal
        open
        tabs={[tab1, tab2]}
        onClose={() => {}}
      />,
    );
    expect(screen.getByText("Bookmarks one")).toBeInTheDocument();
    expect(screen.getByText("Bookmarks two")).toBeInTheDocument();
  });

  it("updates the conflict-strategy state when a radio is selected", async () => {
    const user = userEvent.setup();
    render(
      <MergePickerModal
        open
        tabs={[tab1]}
        onClose={() => {}}
      />,
    );
    const keepFirst = screen.getByLabelText(/keep first/i) as HTMLInputElement;
    const keepBoth = screen.getByLabelText(/keep both/i) as HTMLInputElement;
    expect(keepFirst.checked).toBe(true);

    await user.click(keepBoth);
    expect(keepBoth.checked).toBe(true);
    expect(keepFirst.checked).toBe(false);
  });

  it("enables Merge once a folder is picked", async () => {
    const user = userEvent.setup();
    vi.mocked(ipc.getTree).mockResolvedValue(tree1);

    render(
      <MergePickerModal
        open
        tabs={[tab1]}
        onClose={() => {}}
      />,
    );

    const mergeBtn = screen.getByRole("button", { name: /^merge$/i });
    expect(mergeBtn).toBeDisabled();

    // Expand the tab section to load its tree.
    await user.click(screen.getByRole("button", { name: /Bookmarks one/i }));
    await waitFor(() => expect(ipc.getTree).toHaveBeenCalledWith(1));
    await screen.findByText("Folder A");

    // Pick one folder: the plan lists it by its path under the tab title.
    const pickButtons = screen.getAllByRole("button", { name: /^pick$/i });
    await user.click(pickButtons[0]);

    expect(screen.getByText("Bookmarks one › Folder A")).toBeInTheDocument();
    await waitFor(() => expect(mergeBtn).not.toBeDisabled());
  });
});
