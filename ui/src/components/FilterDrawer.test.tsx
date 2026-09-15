import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { FilterDrawer } from "./FilterDrawer";
import type { FilterSpec } from "../ipc/types";

function renderDrawer(initial: FilterSpec | null = null, showDepth = false) {
  let current: FilterSpec | null = initial;
  const onChange = vi.fn((next: FilterSpec | null) => {
    current = next;
  });
  const utils = render(
    <FilterDrawer filter={current} onChange={onChange} showDepth={showDepth} />,
  );
  return { ...utils, onChange, get: () => current };
}

describe("FilterDrawer", () => {
  it("renders the kind / date / domain / TLD / scheme rows in browse mode", () => {
    renderDrawer();
    expect(screen.getByText("Kind")).toBeInTheDocument();
    expect(screen.getByText("Added")).toBeInTheDocument();
    expect(screen.getByText("Domain")).toBeInTheDocument();
    expect(screen.getByText("TLD")).toBeInTheDocument();
    expect(screen.getByText("Scheme")).toBeInTheDocument();
    // Depth should be hidden when showDepth = false
    expect(screen.queryByText("Depth")).not.toBeInTheDocument();
  });

  it("shows the depth row when showDepth is true (search mode)", () => {
    renderDrawer(null, true);
    expect(screen.getByText("Depth")).toBeInTheDocument();
  });

  it("toggling a kind checkbox emits a kinds[] constraint", async () => {
    const user = userEvent.setup();
    const { onChange } = renderDrawer();

    // Each kind starts checked (no constraint).  Unchecking "Folders" should
    // emit kinds = ["bookmark", "separator"].
    const folderCheckbox = screen.getByRole("checkbox", { name: /folders/i });
    await user.click(folderCheckbox);

    expect(onChange).toHaveBeenCalled();
    const last = onChange.mock.calls.at(-1)?.[0] as FilterSpec | null;
    expect(last?.kinds).toEqual(["bookmark", "separator"]);
  });

  it("typing in the domain chip input and pressing Enter adds a chip", async () => {
    const user = userEvent.setup();
    const { onChange } = renderDrawer();

    const input = screen.getByPlaceholderText(/example\.com/i);
    await user.type(input, "github.com{Enter}");

    const last = onChange.mock.calls.at(-1)?.[0] as FilterSpec | null;
    expect(last?.domains).toEqual(["github.com"]);
  });

  it("comma also commits a chip", async () => {
    const user = userEvent.setup();
    const { onChange } = renderDrawer();

    const input = screen.getByPlaceholderText(/com, org, dev/i);
    await user.type(input, "com,");

    const last = onChange.mock.calls.at(-1)?.[0] as FilterSpec | null;
    expect(last?.tlds).toEqual(["com"]);
  });

  it("Reset all filters clears the spec when something is active", async () => {
    const user = userEvent.setup();
    const { onChange } = renderDrawer({ schemes: ["https"] });

    await user.click(screen.getByText(/reset all filters/i));

    expect(onChange).toHaveBeenCalledWith(null);
  });

  it("does not show the reset link when no filter is active", () => {
    renderDrawer(null);
    expect(screen.queryByText(/reset all filters/i)).not.toBeInTheDocument();
  });
});
