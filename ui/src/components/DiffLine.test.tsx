import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { DiffLine } from "./DiffLine";
import { span } from "../test/fixtures";

describe("DiffLine", () => {
  it("renders the empty-placeholder when `fallback` is blank", () => {
    render(<DiffLine spans={[]} fallback="" side="before" />);
    expect(screen.getByText("empty")).toBeInTheDocument();
  });

  it("falls back to plain text + line-through on the before side when spans are empty", () => {
    render(
      <DiffLine spans={[]} fallback="https://example.com" side="before" />,
    );
    const line = screen.getByText("https://example.com");
    expect(line).toBeInTheDocument();
    expect(line.className).toContain("line-through");
  });

  it("falls back to plain text without strikethrough on the after side", () => {
    render(
      <DiffLine spans={[]} fallback="https://example.com" side="after" />,
    );
    const line = screen.getByText("https://example.com");
    expect(line).toBeInTheDocument();
    expect(line.className).not.toContain("line-through");
  });

  it("falls back when spans are undefined (diff skipped)", () => {
    render(
      <DiffLine spans={undefined} fallback="https://example.com" side="before" />,
    );
    expect(screen.getByText("https://example.com")).toBeInTheDocument();
  });

  it("renders removed spans with strikethrough styling", () => {
    const spans = [span("equal", "https://example.com"), span("removed", "?utm_source=x")];
    render(<DiffLine spans={spans} fallback="https://example.com?utm_source=x" side="before" />);
    const removed = screen.getByText("?utm_source=x");
    expect(removed.className).toContain("line-through");
    expect(removed.className).toContain("diff-removed");
  });

  it("renders added spans with the added-highlight styling", () => {
    const spans = [span("equal", "https://example.com"), span("added", "/path")];
    render(<DiffLine spans={spans} fallback="https://example.com/path" side="after" />);
    const added = screen.getByText("/path");
    expect(added.className).toContain("diff-added");
    expect(added.className).not.toContain("line-through");
  });

  it("preserves span order as the source of truth", () => {
    const spans = [
      span("equal", "a"),
      span("removed", "b"),
      span("equal", "c"),
      span("added", "d"),
    ];
    const { container } = render(
      <DiffLine spans={spans} fallback="abcd" side="after" />,
    );
    const texts = Array.from(container.querySelectorAll("span")).map((s) => s.textContent);
    expect(texts).toEqual(["a", "b", "c", "d"]);
  });
});
