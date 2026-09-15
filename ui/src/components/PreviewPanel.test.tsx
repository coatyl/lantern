import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PreviewPanel } from "./PreviewPanel";
import { change, preview } from "../test/fixtures";

vi.mock("../ipc", () => ({
  ipc: {
    applyChangeset: vi.fn(),
  },
}));

vi.mock("../state/documents", () => ({
  useDocuments: () => ({
    refreshTabs: vi.fn().mockResolvedValue(undefined),
    refreshTree: vi.fn().mockResolvedValue(undefined),
    refreshList: vi.fn().mockResolvedValue(undefined),
  }),
}));

import { ipc } from "../ipc";

describe("PreviewPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the change count, rule-set name, and initial approvals", () => {
    const p = preview({
      rule_set_name: "Minimal clean",
      changes: [change({ approved: true }), change({ approved: false, index: 1 })],
    });
    render(
      <PreviewPanel preview={p} tabId={1} onDone={() => {}} onCancel={() => {}} />,
    );
    expect(screen.getByText("2 proposed changes")).toBeInTheDocument();
    expect(screen.getByText("Minimal clean")).toBeInTheDocument();
    expect(screen.getByText("1 selected")).toBeInTheDocument();
  });

  it("toggling a single row updates the approved count", async () => {
    const user = userEvent.setup();
    const p = preview({
      changes: [change({ approved: true }), change({ approved: true, index: 1 })],
    });
    render(
      <PreviewPanel preview={p} tabId={1} onDone={() => {}} onCancel={() => {}} />,
    );
    const rows = screen.getAllByRole("checkbox");
    await user.click(rows[0]);
    expect(screen.getByText("1 selected")).toBeInTheDocument();
  });

  it("All / None bulk-toggle every row", async () => {
    const user = userEvent.setup();
    const p = preview({
      changes: [
        change({ approved: true }),
        change({ approved: false, index: 1 }),
        change({ approved: true, index: 2 }),
      ],
    });
    render(
      <PreviewPanel preview={p} tabId={1} onDone={() => {}} onCancel={() => {}} />,
    );
    await user.click(screen.getByText("None"));
    expect(screen.getByText("0 selected")).toBeInTheDocument();
    await user.click(screen.getByText("All"));
    expect(screen.getByText("3 selected")).toBeInTheDocument();
  });

  it("disables Apply when nothing is selected", async () => {
    const user = userEvent.setup();
    const p = preview({
      changes: [change({ approved: true }), change({ approved: true, index: 1 })],
    });
    render(
      <PreviewPanel preview={p} tabId={1} onDone={() => {}} onCancel={() => {}} />,
    );
    const apply = screen.getByRole("button", { name: /Apply \d+/ });
    expect(apply).not.toBeDisabled();
    await user.click(screen.getByText("None"));
    expect(apply).toBeDisabled();
  });

  it("calls applyChangeset with the current approvals and onDone with a summary", async () => {
    const user = userEvent.setup();
    vi.mocked(ipc.applyChangeset).mockResolvedValue({
      applied_count: 1,
      skipped_count: 1,
    });
    const onDone = vi.fn();
    const p = preview({
      changeset_id: 42,
      changes: [change({ approved: true }), change({ approved: false, index: 1 })],
    });
    render(
      <PreviewPanel preview={p} tabId={7} onDone={onDone} onCancel={() => {}} />,
    );
    await user.click(screen.getByRole("button", { name: /Apply 1/ }));
    await waitFor(() => expect(onDone).toHaveBeenCalledOnce());
    expect(ipc.applyChangeset).toHaveBeenCalledWith(7, 42, [true, false]);
    expect(onDone).toHaveBeenCalledWith("Applied 1 change, skipped 1.");
  });

  it("fires onCancel when Discard is clicked", async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    render(
      <PreviewPanel preview={preview()} tabId={1} onDone={() => {}} onCancel={onCancel} />,
    );
    await user.click(screen.getByText("Discard"));
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("pluralises correctly for a single change", () => {
    const p = preview({ changes: [change({ approved: true })] });
    render(
      <PreviewPanel preview={p} tabId={1} onDone={() => {}} onCancel={() => {}} />,
    );
    expect(screen.getByText("1 proposed change")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Apply 1 change" })).toBeInTheDocument();
  });
});
