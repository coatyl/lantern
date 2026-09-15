import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";

import { LogsPane } from "./LogsPane";
import { I18nProvider } from "../../i18n/I18nProvider";
import { ipc } from "../../ipc";

vi.mock("../../ipc", () => ({
  ipc: {
    getLogs: vi.fn(),
  },
}));

function renderPane() {
  return render(
    <I18nProvider locale="en">
      <LogsPane />
    </I18nProvider>,
  );
}

beforeEach(() => {
  vi.mocked(ipc.getLogs).mockReset();
});

describe("LogsPane", () => {
  it("renders the empty state when ipc.getLogs returns []", async () => {
    vi.mocked(ipc.getLogs).mockResolvedValue([]);

    renderPane();

    await waitFor(() => {
      expect(screen.getByText(/no log entries yet/i)).toBeInTheDocument();
    });
  });
});
