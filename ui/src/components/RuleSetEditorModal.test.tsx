import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { RuleSetEditorModal } from "./RuleSetEditorModal";
import { ipc } from "../ipc";
import type { RuleSetDetail } from "../ipc/types";

vi.mock("../ipc", () => ({
  ipc: {
    listRuleSets: vi.fn(),
    listTreatments: vi.fn(),
    getRuleSet: vi.fn(),
    saveRuleSetWithConfigs: vi.fn(),
    deleteRuleSet: vi.fn(),
  },
}));

const mine: RuleSetDetail = {
  name: "Mine",
  treatment_ids: ["url.qp.utm"],
  treatments: [{ id: "url.qp.utm", config: null }],
  is_builtin: false,
  path: "",
};

beforeEach(() => {
  vi.mocked(ipc.listRuleSets).mockResolvedValue([
    { name: "Mine", treatment_count: 1, is_builtin: false, path: "" },
  ]);
  vi.mocked(ipc.listTreatments).mockResolvedValue([
    { id: "url.qp.utm", name: "Strip UTM", category: "url_query_param", destructive: false },
    { id: "url.qp.custom", name: "Custom params", category: "url_query_param", destructive: false },
  ]);
  vi.mocked(ipc.getRuleSet).mockResolvedValue(mine);
  vi.mocked(ipc.saveRuleSetWithConfigs).mockReset();
  vi.mocked(ipc.deleteRuleSet).mockReset().mockResolvedValue(undefined);
});

function renderEditor(onClose = vi.fn(), onRefreshList = vi.fn()) {
  render(
    <RuleSetEditorModal open initialSet="Mine" onClose={onClose} onRefreshList={onRefreshList} />,
  );
  return { onClose, onRefreshList };
}

describe("RuleSetEditorModal", () => {
  it("saves added treatments in order, with their config", async () => {
    const user = userEvent.setup();
    const { onRefreshList } = renderEditor();
    await screen.findByRole("heading", { name: "Mine" });

    await user.click(screen.getByRole("button", { name: /add treatment/i }));
    await user.click(screen.getByRole("button", { name: "Custom params" }));
    expect(screen.getByText(/unsaved/)).toBeInTheDocument();
    await user.type(screen.getByPlaceholderText("param_name…"), "fbclid{Enter}");

    vi.mocked(ipc.saveRuleSetWithConfigs).mockResolvedValue({
      ...mine,
      treatment_ids: ["url.qp.utm", "url.qp.custom"],
      treatments: [
        { id: "url.qp.utm", config: null },
        { id: "url.qp.custom", config: { params: ["fbclid"] } },
      ],
    });
    await user.click(screen.getByRole("button", { name: "Save" }));

    expect(ipc.saveRuleSetWithConfigs).toHaveBeenCalledWith("Mine", [
      { id: "url.qp.utm", config: null },
      { id: "url.qp.custom", config: { params: ["fbclid"] } },
    ]);
    expect(onRefreshList).toHaveBeenCalled();
    expect(screen.queryByText(/unsaved/)).not.toBeInTheDocument();
  });

  it("confirms a delete in its own dialog, which Escape dismisses without closing the editor", async () => {
    const user = userEvent.setup();
    const { onClose } = renderEditor();
    await screen.findByRole("heading", { name: "Mine" });

    await user.click(screen.getByTitle("Delete"));
    const confirm = screen.getByRole("dialog", { name: "Delete rule set?" });
    expect(within(confirm).getByRole("button", { name: "Cancel" })).toHaveFocus();

    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog", { name: "Delete rule set?" })).not.toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();

    await user.click(screen.getByTitle("Delete"));
    const again = screen.getByRole("dialog", { name: "Delete rule set?" });
    await user.click(within(again).getByRole("button", { name: "Delete" }));
    expect(ipc.deleteRuleSet).toHaveBeenCalledWith("Mine");
  });
});
