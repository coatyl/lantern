/**
 * TreePane: progressive-expansion tests (v0.0.8 perf hardening, slice 2).
 *
 * The interesting question for these tests is: does the lazy tree actually
 * keep the DOM small for deep documents, and does expanding a row trigger
 * exactly one IPC fetch (with subsequent expand/collapse staying purely
 * local)?  We mount with a synthetic "5 × 5 × 5 = 125 folder" document
 * and assert:
 *
 *   1. only the 5 top-level rows are in the DOM after mount,
 *   2. clicking a row's expand chevron fetches its children once and
 *      renders them, and a subsequent collapse does not refetch.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { act, render, fireEvent, waitFor } from "@testing-library/react";
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

describe("TreePane progressive expansion", () => {
  it("renders only top-level folders on mount even when the tab is huge", async () => {
    const { topLevel, childrenOf } = makeFiveByFive();
    mockGetTreeRoot.mockResolvedValueOnce(topLevel);
    mockGetTreeChildren.mockImplementation((_tab, parentId) =>
      Promise.resolve(childrenOf(parentId)),
    );

    const { container } = renderTree();

    // Wait for the async mount to settle.  After settling, only the 5
    // top-level rows should be in the DOM (the loading spinner is gone).
    await waitFor(() => {
      expect(container.textContent).toContain("Top 1");
      expect(container.textContent).toContain("Top 5");
    });

    // The 25 children should NOT be in the DOM; none of them have been
    // fetched, and even if they had, none of the rows are expanded.
    expect(container.textContent).not.toContain("Child 1.1");
    expect(container.textContent).not.toContain("Child 5.5");
    expect(container.textContent).not.toContain("Grand");

    // And we should have done exactly one IPC call: getTreeRoot.  No
    // children were ever fetched.
    expect(mockGetTreeRoot).toHaveBeenCalledTimes(1);
    expect(mockGetTreeChildren).not.toHaveBeenCalled();
  });

  it("expanding a folder fetches and renders its children exactly once", async () => {
    const { topLevel, childrenOf } = makeFiveByFive();
    mockGetTreeRoot.mockResolvedValueOnce(topLevel);
    mockGetTreeChildren.mockImplementation((_tab, parentId) =>
      Promise.resolve(childrenOf(parentId)),
    );

    const { container } = renderTree();

    await waitFor(() => {
      expect(container.textContent).toContain("Top 2");
    });

    // Find the expand button for "Top 2".  The chevron button has
    // aria-label="Expand" before being toggled.  We pick the button
    // adjacent to "Top 2".
    const top2Folder = Array.from(container.querySelectorAll("button")).find(
      (b) => b.textContent?.includes("Top 2"),
    );
    expect(top2Folder).toBeDefined();
    // The expand chevron is the previous sibling button.
    const expandBtn = top2Folder!.previousElementSibling as HTMLButtonElement;
    expect(expandBtn).not.toBeNull();
    expect(expandBtn.getAttribute("aria-expanded")).toBe("false");

    await act(async () => {
      fireEvent.click(expandBtn);
    });

    // Children of Top 2 (ids 21..25) should now appear.
    await waitFor(() => {
      expect(container.textContent).toContain("Child 2.1");
      expect(container.textContent).toContain("Child 2.5");
    });

    // Children of *other* top-level folders should still be absent.
    expect(container.textContent).not.toContain("Child 1.1");
    expect(container.textContent).not.toContain("Child 3.1");

    // Exactly one getTreeChildren call (for Top 2 only).
    expect(mockGetTreeChildren).toHaveBeenCalledTimes(1);
    expect(mockGetTreeChildren).toHaveBeenCalledWith(1, 2);

    // Collapse → no extra fetch.
    await act(async () => {
      fireEvent.click(expandBtn);
    });
    expect(mockGetTreeChildren).toHaveBeenCalledTimes(1);

    // Re-expand → still no extra fetch (cached).
    await act(async () => {
      fireEvent.click(expandBtn);
    });
    expect(mockGetTreeChildren).toHaveBeenCalledTimes(1);
  });
});

describe("TreePane inline rename (v0.0.11)", () => {
  it("F2 on a focused folder swaps the label for an input pre-filled with the name; Enter calls renameNode", async () => {
    const { topLevel, childrenOf } = makeFiveByFive();
    mockGetTreeRoot.mockResolvedValueOnce(topLevel);
    mockGetTreeChildren.mockImplementation((_tab, parentId) =>
      Promise.resolve(childrenOf(parentId)),
    );
    mockRenameNode.mockResolvedValue(undefined);

    const user = userEvent.setup();
    const { container } = renderTree();

    await waitFor(() => {
      expect(container.textContent).toContain("Top 3");
    });

    // The folder label is the second button per row (the chevron is first).
    const top3Btn = Array.from(container.querySelectorAll("button")).find(
      (b) => b.textContent?.includes("Top 3"),
    ) as HTMLButtonElement | undefined;
    expect(top3Btn).toBeDefined();

    // Focus the folder label and press F2.
    top3Btn!.focus();
    await user.keyboard("{F2}");

    // The label is replaced by an input pre-filled with the folder name.
    const input = await waitFor(() =>
      container.querySelector<HTMLInputElement>("input[data-rename-input]"),
    );
    expect(input).not.toBeNull();
    expect(input!.value).toBe("Top 3");

    // Type a new name and press Enter.
    await user.clear(input!);
    await user.type(input!, "Renamed Folder{Enter}");

    await waitFor(() => {
      expect(mockRenameNode).toHaveBeenCalledWith(1, 3, "Renamed Folder");
    });
  });
});
