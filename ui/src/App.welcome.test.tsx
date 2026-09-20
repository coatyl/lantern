/**
 * Welcome screen: archive-entrance copy and hierarchy.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";

import { WelcomeScreen } from "./App";
import { I18nProvider } from "./i18n/I18nProvider";

vi.mock("./ipc", () => ({
  ipc: {
    listRecentFiles: vi.fn().mockResolvedValue([]),
    getRecoveryState: vi.fn().mockResolvedValue({ paths: [] }),
  },
}));

vi.mock("./state/documents", () => ({
  useDocuments: () => ({
    refreshTabs: vi.fn(),
    setActiveTab: vi.fn(),
  }),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

function renderWelcome() {
  return render(
    <I18nProvider locale="en">
      <WelcomeScreen onOpen={vi.fn()} />
    </I18nProvider>,
  );
}

describe("WelcomeScreen", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the archive entrance: mark, title, manifesto, and open action", () => {
    renderWelcome();

    expect(screen.getByRole("heading", { name: "Lantern" })).toBeInTheDocument();
    expect(
      screen.getByText("A light in a dark room. The archive stays on this machine."),
    ).toBeInTheDocument();
    expect(screen.getByText("Private archive")).toBeInTheDocument();

    const open = screen.getByRole("button", { name: "Open file…" });
    expect(open).toBeInTheDocument();
    expect(open).not.toHaveAttribute("tabindex", "-1");
  });
});
