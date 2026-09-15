/**
 * Tests for useFocusTrap, built as a tiny mounting harness so the hook is
 * exercised through real DOM events instead of unit-stubbing React internals.
 */
import { describe, it, expect, vi } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState, createElement } from "react";

import { useFocusTrap } from "./useFocusTrap";

interface HarnessProps {
  initiallyOpen?: boolean;
  onClose?: () => void;
}

/**
 * Minimal modal-shaped harness that exercises every contract the hook owns:
 * activation, Tab cycling, Escape handling, and focus restoration on unmount.
 */
function Harness({ initiallyOpen = true, onClose }: HarnessProps) {
  const [open, setOpen] = useState(initiallyOpen);
  const handleClose = () => {
    onClose?.();
    setOpen(false);
  };
  const ref = useFocusTrap<HTMLDivElement>(open, handleClose);

  return createElement(
    "div",
    null,
    createElement(
      "button",
      { "data-testid": "trigger", onClick: () => setOpen(true) },
      "Open",
    ),
    open
      ? createElement(
          "div",
          { ref, "data-testid": "dialog", role: "dialog" },
          createElement("button", { "data-testid": "first" }, "First"),
          createElement("button", { "data-testid": "middle" }, "Middle"),
          createElement("button", { "data-testid": "last" }, "Last"),
        )
      : null,
  );
}

describe("useFocusTrap", () => {
  it("wraps Tab from the last focusable element back to the first", async () => {
    const user = userEvent.setup();
    render(createElement(Harness));

    // Hook focuses the first element on mount.
    const first = screen.getByTestId("first");
    const middle = screen.getByTestId("middle");
    const last = screen.getByTestId("last");
    expect(document.activeElement).toBe(first);

    // Tab through the cycle.
    await user.tab();
    expect(document.activeElement).toBe(middle);
    await user.tab();
    expect(document.activeElement).toBe(last);

    // Tab from last must wrap to first (this is the trap behaviour).
    await user.tab();
    expect(document.activeElement).toBe(first);

    // Shift+Tab from first wraps backwards to last.
    await user.tab({ shift: true });
    expect(document.activeElement).toBe(last);
  });

  it("invokes onClose when Escape is pressed inside the dialog", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(createElement(Harness, { onClose }));

    // Focus is inside the dialog; press Escape.
    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("restores focus to the previously-focused element on unmount", async () => {
    const user = userEvent.setup();
    // Render a trigger that opens the dialog.  This way the trigger holds
    // focus when the dialog mounts, and the hook should hand focus back when
    // the dialog unmounts.
    render(createElement(Harness, { initiallyOpen: false }));

    const trigger = screen.getByTestId("trigger");
    trigger.focus();
    expect(document.activeElement).toBe(trigger);

    // Open the dialog by clicking the trigger.  After mount, the hook moves
    // focus to the first focusable element inside the dialog.
    await user.click(trigger);
    expect(document.activeElement).toBe(screen.getByTestId("first"));

    // Pressing Escape closes the dialog (Harness toggles `open` to false on
    // close, which unmounts the dialog and triggers the cleanup path).
    await user.keyboard("{Escape}");

    // Focus must return to the element that was focused before the trap
    // activated (in this case, the trigger button).
    expect(document.activeElement).toBe(trigger);

    cleanup();
  });
});
