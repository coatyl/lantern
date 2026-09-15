import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { SettingsModal } from "./SettingsModal";
import { I18nProvider } from "../i18n/I18nProvider";
import { ipc } from "../ipc";
import type { AppSettings } from "../ipc/types";

function renderModal(props: React.ComponentProps<typeof SettingsModal>) {
  return render(
    <I18nProvider locale="en">
      <SettingsModal {...props} />
    </I18nProvider>,
  );
}

vi.mock("../ipc", () => ({
  ipc: {
    getSettings:    vi.fn(),
    updateSettings: vi.fn(),
    listShortcuts:  vi.fn(),
    getLogs:        vi.fn(),
    getBuildInfo:   vi.fn(),
  },
}));

const stubSettings: AppSettings = {
  theme: "system",
  dead_link_checker_opt_in: false,
  recent_files_max: 10,
  crash_recovery_enabled: true,
  list_density: "compact",
  settings_path: "C:/tmp/settings.toml",
  rules_dir: "C:/tmp/rules",
};

beforeEach(() => {
  vi.mocked(ipc.getSettings).mockReset();
  vi.mocked(ipc.updateSettings).mockReset();
  vi.mocked(ipc.listShortcuts).mockReset();
  vi.mocked(ipc.getLogs).mockReset();
  vi.mocked(ipc.getBuildInfo).mockReset();

  vi.mocked(ipc.getSettings).mockResolvedValue(stubSettings);
  vi.mocked(ipc.listShortcuts).mockResolvedValue([]);
  vi.mocked(ipc.getLogs).mockResolvedValue([]);
  vi.mocked(ipc.getBuildInfo).mockResolvedValue({
    version: "0.0.7",
    build_flavor: "default",
    rust_version: "1.82.0",
    git_commit: null,
    license: "MIT",
    adr_index_path: "adrs/",
    signed: false,
  });
});

describe("SettingsModal", () => {
  it("ArrowDown on the rail moves the active pane", async () => {
    const user = userEvent.setup();
    renderModal({ open: true, onClose: () => {} });

    // Wait for the modal body to render after the async getSettings load.
    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "General" })).toBeInTheDocument();
    });

    const generalTab = screen.getByRole("tab", { name: "General" });
    expect(generalTab).toHaveAttribute("aria-selected", "true");

    generalTab.focus();
    await user.keyboard("{ArrowDown}");

    const keyboardTab = screen.getByRole("tab", { name: "Keyboard" });
    await waitFor(() => {
      expect(keyboardTab).toHaveAttribute("aria-selected", "true");
    });
    expect(generalTab).toHaveAttribute("aria-selected", "false");
  });

  it("only the active rail tab has tabIndex={0}", async () => {
    renderModal({ open: true, onClose: () => {} });

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "General" })).toBeInTheDocument();
    });

    const tabs = screen.getAllByRole("tab");
    expect(tabs).toHaveLength(4);

    const activeTabs = tabs.filter((t) => t.getAttribute("tabindex") === "0");
    const inactiveTabs = tabs.filter((t) => t.getAttribute("tabindex") === "-1");

    expect(activeTabs).toHaveLength(1);
    expect(activeTabs[0]).toHaveAttribute("aria-selected", "true");
    expect(inactiveTabs).toHaveLength(3);
    inactiveTabs.forEach((t) => {
      expect(t).toHaveAttribute("aria-selected", "false");
    });
  });
});
