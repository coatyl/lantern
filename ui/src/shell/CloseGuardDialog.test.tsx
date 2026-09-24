import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import CloseGuardDialog from "./CloseGuardDialog";
import { I18nProvider } from "../i18n/I18nProvider";

const saveCopyOf = vi.fn();
const store = {
  tabs: [
    { id: 1, title: "edited.html", dirty: true },
    { id: 2, title: "clean.html", dirty: false },
  ],
  pendingClose: { tabIds: [1, 2], closeWindow: false } as { tabIds: number[]; closeWindow: boolean } | null,
  confirmClose: vi.fn(),
  cancelClose: vi.fn(),
};

vi.mock("../state/documents", () => ({ useDocuments: () => store }));
vi.mock("../state/fileActions", () => ({ useFileActions: () => ({ saveCopyOf }) }));

const renderGuard = () =>
  render(
    <I18nProvider locale="en">
      <CloseGuardDialog />
    </I18nProvider>,
  );

beforeEach(() => {
  vi.clearAllMocks();
  store.pendingClose = { tabIds: [1, 2], closeWindow: false };
});

describe("CloseGuardDialog", () => {
  it("lists only the edited tabs", () => {
    renderGuard();
    expect(screen.getByRole("dialog", { name: "Close with unsaved edits?" })).toBeInTheDocument();
    expect(screen.getByText("edited.html")).toBeInTheDocument();
    expect(screen.queryByText("clean.html")).not.toBeInTheDocument();
  });

  it("closes only after every copy is saved", async () => {
    const user = userEvent.setup();
    saveCopyOf.mockResolvedValueOnce(false);
    renderGuard();
    await user.click(screen.getByRole("button", { name: "Save copy and close" }));
    expect(store.confirmClose).not.toHaveBeenCalled();

    saveCopyOf.mockResolvedValueOnce(true);
    await user.click(screen.getByRole("button", { name: "Save copy and close" }));
    expect(saveCopyOf).toHaveBeenLastCalledWith(1);
    expect(store.confirmClose).toHaveBeenCalledTimes(1);
  });

  it("discards or cancels on request", async () => {
    const user = userEvent.setup();
    renderGuard();
    await user.click(screen.getByRole("button", { name: "Discard edits" }));
    expect(store.confirmClose).toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(store.cancelClose).toHaveBeenCalled();
  });

  it("renders nothing when no edited tab is pending", () => {
    store.pendingClose = { tabIds: [2], closeWindow: false };
    const { container } = renderGuard();
    expect(container).toBeEmptyDOMElement();
  });
});
