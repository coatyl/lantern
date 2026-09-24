/**
 * EmptyState: v0.0.11 QoL slice 1.
 *
 * Two cases:
 *   1. Title + description render.
 *   2. The optional action button calls its `onClick` when activated.
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { EmptyState } from "./EmptyState";

describe("EmptyState", () => {
  it("renders title and description", () => {
    render(
      <EmptyState
        title="No items"
        description="Add one to see it here."
      />,
    );
    expect(screen.getByRole("region", { name: "No items" })).toBeInTheDocument();
    expect(screen.getByText("No items")).toBeInTheDocument();
    expect(
      screen.getByText("Add one to see it here."),
    ).toBeInTheDocument();
  });

  it("invokes the action onClick when the button is activated", async () => {
    const user = userEvent.setup();
    const handler = vi.fn();
    render(
      <EmptyState
        title="No items"
        action={{ label: "Add one", onClick: handler }}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Add one" }));
    expect(handler).toHaveBeenCalledOnce();
  });
});
