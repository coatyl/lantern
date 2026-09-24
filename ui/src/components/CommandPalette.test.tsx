import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";

import { CommandPalette } from "./CommandPalette";
import { filterCommands, stepActiveIndex, type PaletteCommand } from "./paletteCommands";
import { I18nProvider } from "../i18n/I18nProvider";

const COMMANDS: PaletteCommand[] = [
  { id: "open_file", label: "Open file", enabled: true },
  { id: "settings", label: "Settings", enabled: true },
  { id: "save_copy", label: "Save copy", enabled: false },
  { id: "toggle_theme", label: "Toggle theme", enabled: true },
];

function renderPalette(
  props: Partial<ComponentProps<typeof CommandPalette>> = {},
) {
  const onRun = props.onRun ?? vi.fn();
  const onClose = props.onClose ?? vi.fn();
  render(
    <I18nProvider locale="en">
      <CommandPalette
        open
        commands={COMMANDS}
        onRun={onRun}
        onClose={onClose}
        {...props}
      />
    </I18nProvider>,
  );
  return { onRun, onClose };
}

describe("filterCommands", () => {
  it("returns every command when the query is empty or whitespace", () => {
    expect(filterCommands(COMMANDS, "")).toHaveLength(COMMANDS.length);
    expect(filterCommands(COMMANDS, "   ")).toHaveLength(COMMANDS.length);
  });

  it("matches a case-insensitive substring of the label", () => {
    const hits = filterCommands(COMMANDS, "set");
    expect(hits.map((c) => c.id)).toEqual(["settings"]);
  });

  it("matches a subsequence (fuzzy) of the label", () => {
    const hits = filterCommands(COMMANDS, "thm");
    expect(hits.map((c) => c.id)).toEqual(["toggle_theme"]);
  });

  it("returns an empty list when nothing matches", () => {
    expect(filterCommands(COMMANDS, "zzzz")).toHaveLength(0);
  });
});

describe("stepActiveIndex", () => {
  it("wraps from the last row to the first and back", () => {
    expect(stepActiveIndex(2, 1, 3)).toBe(0);
    expect(stepActiveIndex(0, -1, 3)).toBe(2);
  });

  it("stays at 0 when the list is empty", () => {
    expect(stepActiveIndex(4, 1, 0)).toBe(0);
  });
});

describe("CommandPalette keyboard selection", () => {
  it("highlights the first row and runs it on Enter", async () => {
    const user = userEvent.setup();
    const { onRun } = renderPalette();

    expect(screen.getByRole("combobox")).toHaveFocus();
    expect(screen.getByRole("option", { name: "Open file" })).toHaveAttribute(
      "aria-selected",
      "true",
    );

    await user.keyboard("{Enter}");
    expect(onRun).toHaveBeenCalledWith("open_file");
  });

  it("ArrowDown then Enter runs the newly highlighted command", async () => {
    const user = userEvent.setup();
    const { onRun } = renderPalette();

    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("option", { name: "Settings" })).toHaveAttribute(
      "aria-selected",
      "true",
    );

    await user.keyboard("{Enter}");
    expect(onRun).toHaveBeenCalledWith("settings");
  });

  it("filters by label and Enter runs the remaining match", async () => {
    const user = userEvent.setup();
    const { onRun } = renderPalette();

    await user.type(screen.getByRole("combobox"), "set");

    expect(screen.getByRole("option", { name: "Settings" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "Open file" })).not.toBeInTheDocument();

    await user.keyboard("{Enter}");
    expect(onRun).toHaveBeenCalledWith("settings");
  });

  it("does not run a disabled highlighted command", async () => {
    const user = userEvent.setup();
    const { onRun } = renderPalette();

    await user.type(screen.getByRole("combobox"), "save");
    expect(screen.getByRole("option", { name: "Save copy" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );

    await user.keyboard("{Enter}");
    expect(onRun).not.toHaveBeenCalled();
  });

  it("Escape closes the palette", async () => {
    const user = userEvent.setup();
    const { onClose } = renderPalette();

    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
