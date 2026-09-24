/**
 * TreePane: lazy loading (only fetched when expanded, fetched once) and the
 * ARIA tree keyboard model, against a synthetic 5 × 5 × 5 folder document.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { act, render, fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nProvider } from "../i18n/I18nProvider";
import type { TreeNodeLazy } from "../ipc/types";

// ─── Stub the IPC layer.  The mock is hoisted; we drive `getTreeRoot`
//     and `getTreeChildren` per test by swapping the impls before
//     mounting. ──────────────────────────────────────────────────────
const mockGetTreeRoot = vi.fn<[number], Promise<TreeNodeLazy[]>>();
const mockGetTreeChildren = vi.fn<[number, number], Promise<TreeNodeLazy[]>>();
const mockRenameNode = vi.fn<[number, number, string], Promise<void>>();

vi.mock("../ipc", () => ({
  ipc: {
    getTreeRoot: (tabId: number) => mockGetTreeRoot(tabId),
    getTreeChildren: (tabId: number, parentId: number) =>
      mockGetTreeChildren(tabId, parentId),
    renameNode: (tabId: number, nodeId: number, newName: string) =>
      mockRenameNode(tabId, nodeId, newName),
  },
}));

// ─── Stub the documents store. ──────────────────────────────────────
interface StoreSnapshot {
  activeTab: number | null;
  folderStack: { id: number; name: string }[];
  treeVersion: number;
  jumpToFolder: (crumbs: { id: number; name: string }[]) => Promise<void>;
  refreshTree: () => Promise<void>;
}

let storeState: StoreSnapshot;

vi.mock("../state/documents", () => {
  const useDocuments = (() => storeState) as unknown as {
    (): StoreSnapshot;
    setState: (...args: unknown[]) => void;
    getState: () => StoreSnapshot;
  };
  useDocuments.setState = vi.fn();
  useDocuments.getState = () => storeState;
  return {
    useDocuments,
  };
});

// Imported *after* the mocks so the module-level closure picks them up.
import TreePane from "./TreePane";

// ─── Test data builders ────────────────────────────────────────────

/** Build the "5 top-levels, each with 5 children, each with 5
 *  grandchildren" id space.  IDs are pre-allocated so the tests can
 *  reference them by formula. */
function makeFiveByFive(): {
  topLevel: TreeNodeLazy[];
  childrenOf: (parentId: number) => TreeNodeLazy[];
  totalFolders: number;
} {
  // Top: 1..5, children of N: N*10+1..N*10+5, grandchildren: parent*100+1..+5.
  const topLevel: TreeNodeLazy[] = Array.from({ length: 5 }, (_, i) => ({
    id: i + 1,
    name: `Top ${i + 1}`,
    has_children: true,
  }));
  function childrenOf(parentId: number): TreeNodeLazy[] {
    if (parentId >= 1 && parentId <= 5) {
      // Top-level → children at parent*10+1..+5, themselves with children.
      return Array.from({ length: 5 }, (_, i) => ({
        id: parentId * 10 + (i + 1),
        name: `Child ${parentId}.${i + 1}`,
        has_children: true,
      }));
    }
    // Anything that looks like a "child" id (10+) → leaves.
    return Array.from({ length: 5 }, (_, i) => ({
      id: parentId * 100 + (i + 1),
      name: `Grand ${parentId}.${i + 1}`,
      has_children: false,
    }));
  }
  // 5 top + 5*5 children + 5*5*5 grandchildren = 5 + 25 + 125 = 155
  return { topLevel, childrenOf, totalFolders: 155 };
}

beforeEach(() => {
  vi.clearAllMocks();
  storeState = {
    activeTab: 1,
    folderStack: [],
    treeVersion: 0,
    jumpToFolder: vi.fn().mockResolvedValue(undefined),
    refreshTree: vi.fn().mockResolvedValue(undefined),
  };
});

function renderTree() {
  return render(
    <I18nProvider locale="en">
      <TreePane />
    </I18nProvider>,
  );
}

const item = (name: string) => screen.getByRole("treeitem", { name: new RegExp(`^${name}$`) });

async function mountFiveByFive() {
  const { topLevel, childrenOf } = makeFiveByFive();
  mockGetTreeRoot.mockResolvedValueOnce(topLevel);
  mockGetTreeChildren.mockImplementation((_tab, parentId) => Promise.resolve(childrenOf(parentId)));
  const view = renderTree();
  await screen.findByRole("treeitem", { name: "Top 5" });
  return view;
}

describe("TreePane lazy loading", () => {
  it("mounts with only the top level and one IPC call", async () => {
    await mountFiveByFive();
    expect(screen.getByRole("tree", { name: "Folders" })).toBeInTheDocument();
    // Root + 5 top-level folders; nothing below is fetched or rendered.
    expect(screen.getAllByRole("treeitem")).toHaveLength(6);
    expect(mockGetTreeRoot).toHaveBeenCalledTimes(1);
    expect(mockGetTreeChildren).not.toHaveBeenCalled();
  });

  it("expanding fetches children once; collapse and re-expand stay local", async () => {
    await mountFiveByFive();
    const top2 = item("Top 2");
    expect(top2).toHaveAttribute("aria-expanded", "false");
    const chevron = top2.querySelector("[aria-hidden]") as HTMLElement;

    await act(async () => {
      fireEvent.click(chevron);
    });
    await screen.findByRole("treeitem", { name: "Child 2.5" });
    expect(item("Top 2")).toHaveAttribute("aria-expanded", "true");
    expect(item("Child 2.1")).toHaveAttribute("aria-level", "3");
    expect(screen.queryByRole("treeitem", { name: "Child 1.1" })).not.toBeInTheDocument();
    expect(mockGetTreeChildren).toHaveBeenCalledWith(1, 2);

    await act(async () => {
      fireEvent.click(chevron);
    });
    await act(async () => {
      fireEvent.click(chevron);
    });
    expect(mockGetTreeChildren).toHaveBeenCalledTimes(1);
  });

  it("clicking a folder shows it in the list with its full path", async () => {
    await mountFiveByFive();
    const user = userEvent.setup();
    await user.click(item("Top 2").querySelector("[aria-hidden]") as HTMLElement);
    await user.click(await screen.findByRole("treeitem", { name: "Child 2.3" }));
    expect(storeState.jumpToFolder).toHaveBeenLastCalledWith([
      { id: 2, name: "Top 2" },
      { id: 23, name: "Child 2.3" },
    ]);
  });
});

describe("TreePane keyboard", () => {
  it("is a single tab stop and supports arrows, Home/End and Enter", async () => {
    await mountFiveByFive();
    const user = userEvent.setup();
    const focusable = screen.getAllByRole("treeitem").filter((el) => el.tabIndex === 0);
    expect(focusable).toHaveLength(1);

    item("All bookmarks").focus();
    await user.keyboard("{ArrowDown}");
    expect(item("Top 1")).toHaveFocus();
    await user.keyboard("{ArrowRight}"); // expand
    await screen.findByRole("treeitem", { name: "Child 1.1" });
    await user.keyboard("{ArrowRight}"); // into first child
    expect(item("Child 1.1")).toHaveFocus();
    await user.keyboard("{ArrowLeft}"); // back to parent
    expect(item("Top 1")).toHaveFocus();
    await user.keyboard("{ArrowLeft}"); // collapse
    expect(item("Top 1")).toHaveAttribute("aria-expanded", "false");
    await user.keyboard("{End}");
    expect(item("Top 5")).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(storeState.jumpToFolder).toHaveBeenLastCalledWith([{ id: 5, name: "Top 5" }]);
    await user.keyboard("{Home}");
    expect(item("All bookmarks")).toHaveFocus();
  });

  it("F2 renames the focused folder; Enter commits", async () => {
    await mountFiveByFive();
    mockRenameNode.mockResolvedValue(undefined);
    const user = userEvent.setup();
    item("Top 3").focus();
    await user.keyboard("{F2}");
    const input = screen.getByRole("textbox", { name: "New name…" }) as HTMLInputElement;
    expect(input.value).toBe("Top 3");
    await user.clear(input);
    await user.type(input, "Renamed Folder{Enter}");
    await waitFor(() => expect(mockRenameNode).toHaveBeenCalledWith(1, 3, "Renamed Folder"));
  });
});
