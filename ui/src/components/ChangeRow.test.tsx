import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ChangeRow } from "./ChangeRow";
import { change } from "../test/fixtures";

describe("ChangeRow", () => {
  it("renders the field label, rationale, and before/after values", () => {
    render(
      <ChangeRow
        change={change({ field: "title", rationale: "Trim whitespace" })}
        approved
        onToggle={() => {}}
      />,
    );
    expect(screen.getByText("title")).toBeInTheDocument();
    expect(screen.getByText("Trim whitespace")).toBeInTheDocument();
  });

  it("reflects the approved state in the checkbox", () => {
    const { rerender } = render(
      <ChangeRow change={change()} approved onToggle={() => {}} />,
    );
    expect(screen.getByRole("checkbox")).toBeChecked();

    rerender(<ChangeRow change={change()} approved={false} onToggle={() => {}} />);
    expect(screen.getByRole("checkbox")).not.toBeChecked();
  });

  it("fires onToggle when the checkbox is clicked", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(<ChangeRow change={change()} approved={false} onToggle={onToggle} />);
    await user.click(screen.getByRole("checkbox"));
    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  it("shows the `destructive` pill only for destructive changes", () => {
    const { rerender } = render(
      <ChangeRow change={change({ destructive: false })} approved onToggle={() => {}} />,
    );
    expect(screen.queryByText("destructive")).not.toBeInTheDocument();

    rerender(
      <ChangeRow change={change({ destructive: true })} approved onToggle={() => {}} />,
    );
    expect(screen.getByText("destructive")).toBeInTheDocument();
  });
});
