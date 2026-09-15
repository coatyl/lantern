/**
 * TabBar: pointer-extras tests for v0.0.11 QoL slice 2.
 *
 *   1. Middle-click on a tab calls `closeTab` with that tab's id.
 *   2. Right-click opens the context menu (one menuitem visible).
 *   3. Selecting "Close other tabs" closes every tab except the right-clicked one.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import TabBar from "./TabBar";
import { I18nProvider } from "../i18n/I18nProvider";

interface TabSnapshot {
  id: number;
  title: string;
  dirty: boolean;
}

interface StoreSnapshot {
  tabs: TabSnapshot[];
  activeTab: number | null;
  setActiveTab: (id: number) => void;
  closeTab: (id: number) => void;
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
  return { useDocuments };
});

function renderTabBar() {
  return render(
    <I18nProvider locale="en">
      <TabBar />
    </I18nProvider>,
  );
}

beforeEach(() => {
  storeState = {
    tabs: [
      { id: 1, title: "Alpha", dirty: false },
      { id: 2, title: "Beta",  dirty: false },
      { id: 3, title: "Gamma", dirty: false },
    ],
    activeTab: 1,
    setActiveTab: vi.fn(),
    closeTab: vi.fn(),
  };
});

describe("TabBar: pointer extras (v0.0.11)", () => {
  it("middle-click on a tab calls closeTab with that tab's id", () => {
    renderTabBar();

    const betaTab = screen.getByRole("tab", { name: /Beta/ });
    // Middle-click is mouse button 1.  Testing-library doesn't expose an
    // `auxClick` helper directly, so dispatch a synthetic MouseEvent.
    fireEvent(
      betaTab,
      new MouseEvent("auxclick", { button: 1, bubbles: true, cancelable: true }),
    );

    expect(storeState.closeTab).toHaveBeenCalledTimes(1);
    expect(storeState.closeTab).toHaveBeenCalledWith(2);
  });

  it("right-click opens the context menu", async () => {
    renderTabBar();

    const betaTab = screen.getByRole("tab", { name: /Beta/ });
    fireEvent.contextMenu(betaTab, { clientX: 50, clientY: 50 });

    // The popover uses role=menu; "Close tab" is one of the items.
    await waitFor(() => {
      expect(screen.getByRole("menu")).toBeInTheDocument();
    });
    const items = screen.getAllByRole("menuitem");
    expect(items.length).toBeGreaterThan(0);
    expect(items.map((el) => el.textContent)).toContain("Close tab");
  });

  it("'Close other tabs' closes every tab except the right-clicked one", async () => {
    const user = userEvent.setup();
    renderTabBar();

    const betaTab = screen.getByRole("tab", { name: /Beta/ });
    fireEvent.contextMenu(betaTab, { clientX: 50, clientY: 50 });

    await waitFor(() => expect(screen.getByRole("menu")).toBeInTheDocument());

    const closeOthers = await screen.findByRole("menuitem", {
      name: /Close other tabs/,
    });
    await user.click(closeOthers);

    // closeTab called for every tab except the right-clicked id (2).
    const calls = (storeState.closeTab as ReturnType<typeof vi.fn>).mock.calls.flat();
    expect(calls).toContain(1);
    expect(calls).toContain(3);
    expect(calls).not.toContain(2);
    expect(calls).toHaveLength(2);
  });
});
