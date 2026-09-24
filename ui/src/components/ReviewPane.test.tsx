import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { ReviewPane, groupByNode, kindOf } from "./ReviewPane";
import { I18nProvider } from "../i18n/I18nProvider";
import { change, preview } from "../test/fixtures";
import type { PendingReview } from "../state/documents";

const closeReview = vi.fn();
const refresh = vi.fn().mockResolvedValue(undefined);
const toast = vi.fn();

vi.mock("../ipc", () => ({
  ipc: { applyChangeset: vi.fn(), undo: vi.fn() },
}));
vi.mock("../state/documents", () => ({
  useDocuments: () => ({
    refreshTabs: refresh,
    refreshTree: refresh,
    refreshList: refresh,
    closeReview,
  }),
}));
vi.mock("../hooks/useToast", () => ({ useToast: () => ({ toast }) }));

import { ipc } from "../ipc";

const urlChange = change({ index: 0, node_id: 1, node_title: "Docs", location: ["Work", "Refs"] });
const titleChange = change({
  index: 1,
  node_id: 1,
  field: "title",
  before: "Docs | Site",
  after: "Docs",
  before_spans: [],
  after_spans: [],
  rationale: "Removed site suffix",
});
const deleteChange = change({
  index: 2,
  node_id: 7,
  field: "node",
  before: "",
  after: "(deleted)",
  destructive: true,
  approved: false,
  node_title: "Docs (copy)",
  node_url: "https://example.com/docs",
  rationale: "Exact duplicate",
});

function renderReview(changes = [urlChange, titleChange, deleteChange]) {
  const review: PendingReview = {
    tabId: 1,
    preview: preview({ rule_set_name: "Aggressive scrub", changes }),
    scopeLabel: "Whole document",
  };
  return render(
    <I18nProvider locale="en">
      <ReviewPane review={review} />
    </I18nProvider>,
  );
}

describe("groupByNode / kindOf", () => {
  it("groups changes by node in first-seen order", () => {
    const groups = groupByNode([urlChange, deleteChange, titleChange]);
    expect(groups.map((g) => g.nodeId)).toEqual([1, 7]);
    expect(groups[0].changes).toHaveLength(2);
  });

  it("buckets fields into filter kinds", () => {
    expect(kindOf(urlChange)).toBe("url");
    expect(kindOf(titleChange)).toBe("title");
    expect(kindOf(deleteChange)).toBe("node");
  });
});

describe("ReviewPane", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows one card per node with its folder path, and honours initial approvals", () => {
    renderReview();
    expect(screen.getByRole("region", { name: "Proposed changes" })).toBeInTheDocument();
    expect(screen.getAllByText("Docs")[0]).toBeInTheDocument();
    expect(screen.getByText("Work › Refs")).toBeInTheDocument();
    expect(screen.getByText("Docs (copy)")).toBeInTheDocument();
    expect(screen.getByText("2 of 3 selected")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Apply 2 changes" })).toBeEnabled();
  });

  it("names deletions on the Apply button once one is selected", async () => {
    const user = userEvent.setup();
    renderReview();
    await user.click(screen.getByRole("checkbox", { name: "Delete: Exact duplicate" }));
    expect(screen.getByRole("button", { name: "Apply 3 changes (1 deletion)" })).toBeInTheDocument();
  });

  it("kind chips filter the list and bulk actions only touch what is shown", async () => {
    const user = userEvent.setup();
    renderReview();
    await user.click(screen.getByRole("button", { name: /^Title/ }));
    expect(screen.queryByText("Docs (copy)")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Clear shown" }));
    // Only the title change was cleared; the URL change stays selected.
    expect(screen.getByText("1 of 3 selected")).toBeInTheDocument();
  });

  it("applies the approvals, closes the review, and offers Undo", async () => {
    const user = userEvent.setup();
    vi.mocked(ipc.applyChangeset).mockResolvedValue({ applied_count: 2, skipped_count: 1 });
    renderReview();
    await user.click(screen.getByRole("button", { name: "Apply 2 changes" }));
    expect(ipc.applyChangeset).toHaveBeenCalledWith(1, 1, [true, true, false]);
    expect(closeReview).toHaveBeenCalled();
    expect(toast).toHaveBeenCalledWith(
      "Applied 2 changes.",
      "success",
      expect.objectContaining({ label: "Undo" }),
    );
  });

  it("keeps the review open when apply fails", async () => {
    const user = userEvent.setup();
    vi.mocked(ipc.applyChangeset).mockRejectedValue(new Error("boom"));
    renderReview();
    await user.click(screen.getByRole("button", { name: "Apply 2 changes" }));
    expect(toast).toHaveBeenCalledWith("Error: boom", "error");
    expect(closeReview).not.toHaveBeenCalled();
  });

  it("Escape discards; Ctrl+Enter applies", async () => {
    const user = userEvent.setup();
    vi.mocked(ipc.applyChangeset).mockResolvedValue({ applied_count: 2, skipped_count: 1 });
    renderReview();
    const region = screen.getByRole("region", { name: "Proposed changes" });
    within(region).getByRole("heading", { name: "Review changes" }).focus();
    await user.keyboard("{Control>}{Enter}{/Control}");
    expect(ipc.applyChangeset).toHaveBeenCalledTimes(1);

    closeReview.mockClear();
    renderReview();
    screen.getAllByRole("heading", { name: "Review changes" }).at(-1)!.focus();
    await user.keyboard("{Escape}");
    expect(closeReview).toHaveBeenCalled();
  });
});
