/**
 * ContextMenu: generic popover primitive used by the tab-bar context menu
 * and any future right-click surface.  These tests pin three behaviours:
 *
 *   1. Items render as `role="menuitem"` (and only enabled ones do).
 *   2. ArrowDown / ArrowUp move focus through enabled items, wrapping.
 *   3. Escape closes the menu (calls `onClose`).
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { ContextMenu } from "./ContextMenu";

function renderMenu(onClose = vi.fn()) {
  const onA = vi.fn();
  const onB = vi.fn();
  const onC = vi.fn();
  render(
    <ContextMenu
      x={20}
      y={20}
      onClose={onClose}
      ariaLabel="Test menu"
      items={[
        { id: "a", label: "Alpha",   onSelect: onA },
        { id: "b", label: "Beta",    onSelect: onB },
        { id: "c", label: "Gamma",   onSelect: onC },
      ]}
    />,
  );
  return { onA, onB, onC, onClose };
}

describe("ContextMenu", () => {
  it("renders one menuitem per non-separator item", () => {
    renderMenu();
    const items = screen.getAllByRole("menuitem");
    expect(items).toHaveLength(3);
    expect(items[0]).toHaveTextContent("Alpha");
    expect(items[1]).toHaveTextContent("Beta");
    expect(items[2]).toHaveTextContent("Gamma");
  });

  it("ArrowDown / ArrowUp move focus through items with wrap", async () => {
    const user = userEvent.setup();
    renderMenu();

    const items = screen.getAllByRole("menuitem");
    // First item is auto-focused after mount.
    await waitFor(() => expect(items[0]).toHaveFocus());

    await user.keyboard("{ArrowDown}");
    expect(items[1]).toHaveFocus();

    await user.keyboard("{ArrowDown}");
    expect(items[2]).toHaveFocus();

    // Wrap forward.
    await user.keyboard("{ArrowDown}");
    expect(items[0]).toHaveFocus();

    // Wrap backward.
    await user.keyboard("{ArrowUp}");
    expect(items[2]).toHaveFocus();
  });

  it("Escape closes the menu (calls onClose)", async () => {
    const user = userEvent.setup();
    const { onClose } = renderMenu();

    // Focus is inside the menu after mount.
    await waitFor(() => expect(screen.getAllByRole("menuitem")[0]).toHaveFocus());

    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
