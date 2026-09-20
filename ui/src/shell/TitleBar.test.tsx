import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import TitleBar from "./TitleBar";
import { I18nProvider } from "../i18n/I18nProvider";

vi.mock("../ipc", () => ({
  ipc: {
    export: vi.fn(),
  },
}));

interface TitleBarStore {
  activeTab: number | null;
  tabs: { id: number; title: string; path: string | null; dirty: boolean }[];
  showLibrary: () => void;
}

let storeState: TitleBarStore;

vi.mock("../state/documents", () => ({
  useDocuments: () => storeState,
}));

function renderTitleBar(props: Partial<React.ComponentProps<typeof TitleBar>> = {}) {
  return render(
    <I18nProvider locale="en">
      <TitleBar
        onSettings={() => {}}
        onCompareTabs={() => {}}
        onCheckDeadLinks={() => {}}
        onMergeDocuments={() => {}}
        canCompareTabs
        canCheckDeadLinks
        canMergeDocuments
        {...props}
      />
    </I18nProvider>,
  );
}

describe("TitleBar: Tools menu keyboard accessibility", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    storeState = {
      activeTab: null,
      tabs: [],
      showLibrary: vi.fn(),
    };
  });

  it("opens the Tools menu when Space is pressed on the trigger", async () => {
    const user = userEvent.setup();
    renderTitleBar();

    const trigger = screen.getByRole("button", { name: "Tools" });
    expect(trigger).toHaveAttribute("aria-haspopup", "menu");
    expect(trigger).toHaveAttribute("aria-expanded", "false");

    trigger.focus();
    await user.keyboard(" ");

    const menu = await screen.findByRole("menu");
    expect(menu).toBeInTheDocument();
    expect(trigger).toHaveAttribute("aria-expanded", "true");

    const items = screen.getAllByRole("menuitem");
    expect(items).toHaveLength(3);
    // First item should receive focus on open.
    await waitFor(() => expect(items[0]).toHaveFocus());
  });

  it("opens the menu and Enter triggers Down to advance focus, with wraparound", async () => {
    const user = userEvent.setup();
    renderTitleBar();

    const trigger = screen.getByRole("button", { name: "Tools" });
    trigger.focus();
    await user.keyboard("{Enter}");

    const items = screen.getAllByRole("menuitem");
    await waitFor(() => expect(items[0]).toHaveFocus());

    // Arrow Down moves to next item.
    await user.keyboard("{ArrowDown}");
    expect(items[1]).toHaveFocus();

    // Two more downs to reach the end then wrap back to the first.
    await user.keyboard("{ArrowDown}");
    expect(items[2]).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(items[0]).toHaveFocus();

    // Arrow Up wraps the other way.
    await user.keyboard("{ArrowUp}");
    expect(items[items.length - 1]).toHaveFocus();
  });

  it("Escape closes the menu and restores focus to the trigger", async () => {
    const user = userEvent.setup();
    renderTitleBar();

    const trigger = screen.getByRole("button", { name: "Tools" });
    trigger.focus();
    await user.keyboard(" ");

    expect(await screen.findByRole("menu")).toBeInTheDocument();

    await user.keyboard("{Escape}");

    await waitFor(() => expect(screen.queryByRole("menu")).not.toBeInTheDocument());
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it("Home / End jump focus to the first / last menu item", async () => {
    const user = userEvent.setup();
    renderTitleBar();

    const trigger = screen.getByRole("button", { name: "Tools" });
    trigger.focus();
    await user.keyboard(" ");

    const items = screen.getAllByRole("menuitem");
    await waitFor(() => expect(items[0]).toHaveFocus());

    await user.keyboard("{End}");
    expect(items[items.length - 1]).toHaveFocus();

    await user.keyboard("{Home}");
    expect(items[0]).toHaveFocus();
  });
});

describe("TitleBar: Library", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    storeState = {
      activeTab: 1,
      tabs: [{ id: 1, title: "small.html", path: "C:/tmp/small.html", dirty: false }],
      showLibrary: vi.fn(),
    };
  });

  it("shows a Library control that returns home without closing tabs", async () => {
    const user = userEvent.setup();
    renderTitleBar();

    const library = screen.getByRole("button", { name: "Library" });
    expect(library).toBeEnabled();
    await user.click(library);
    expect(storeState.showLibrary).toHaveBeenCalledTimes(1);
  });

  it("hides Library when no volumes are open", () => {
    storeState = {
      activeTab: null,
      tabs: [],
      showLibrary: vi.fn(),
    };
    renderTitleBar();
    expect(screen.queryByRole("button", { name: "Library" })).not.toBeInTheDocument();
  });
});
