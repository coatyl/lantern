import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";

import { KeyboardPane } from "./KeyboardPane";
import { I18nProvider } from "../../i18n/I18nProvider";
import { ipc } from "../../ipc";
import type { ShortcutBinding } from "../../ipc/types";

vi.mock("../../ipc", () => ({
  ipc: {
    listShortcuts: vi.fn(),
  },
}));

function renderPane() {
  return render(
    <I18nProvider locale="en">
      <KeyboardPane />
    </I18nProvider>,
  );
}

const sample: ShortcutBinding[] = [
  { action_id: "open_file",  label: "Open file",  key_combo: "Ctrl+O", category: "File" },
  { action_id: "save_file",  label: "Save",       key_combo: "Ctrl+S", category: "File" },
  { action_id: "find",       label: "Find",       key_combo: "Ctrl+F", category: "Edit" },
];

beforeEach(() => {
  vi.mocked(ipc.listShortcuts).mockReset();
});

describe("KeyboardPane", () => {
  it("renders shortcut rows when ipc.listShortcuts resolves", async () => {
    vi.mocked(ipc.listShortcuts).mockResolvedValue(sample);

    renderPane();

    await waitFor(() => {
      expect(screen.getByText("Open file")).toBeInTheDocument();
    });
    expect(screen.getByText("Save")).toBeInTheDocument();
    expect(screen.getByText("Find")).toBeInTheDocument();
    expect(screen.getByText("Ctrl+O")).toBeInTheDocument();
    // Footer hint about future rebinding work is always present.
    expect(
      screen.getByText(/rebinding will arrive in a future release/i),
    ).toBeInTheDocument();
  });
});
