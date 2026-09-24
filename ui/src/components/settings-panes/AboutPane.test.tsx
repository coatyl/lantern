import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";

import { AboutPane } from "./AboutPane";
import { I18nProvider } from "../../i18n/I18nProvider";
import { ipc } from "../../ipc";
import type { BuildInfo } from "../../ipc/types";

vi.mock("../../ipc", () => ({
  ipc: {
    getBuildInfo: vi.fn(),
  },
}));

function renderPane() {
  return render(
    <I18nProvider locale="en">
      <AboutPane />
    </I18nProvider>,
  );
}

const sample: BuildInfo = {
  version: "0.0.7",
  build_flavor: "default",
  rust_version: "1.82.0",
  git_commit: null,
  license: "MIT",
  signed: false,
};

beforeEach(() => {
  vi.mocked(ipc.getBuildInfo).mockReset();
});

describe("AboutPane", () => {
  it("renders the version when ipc.getBuildInfo resolves", async () => {
    vi.mocked(ipc.getBuildInfo).mockResolvedValue(sample);

    renderPane();

    await waitFor(() => {
      expect(screen.getByText("0.0.7")).toBeInTheDocument();
    });
    expect(screen.getByText("default")).toBeInTheDocument();
    expect(screen.getByText("MIT")).toBeInTheDocument();
  });

  it("links to the project's releases", async () => {
    vi.mocked(ipc.getBuildInfo).mockResolvedValue(sample);

    renderPane();

    const link = await screen.findByRole("link", { name: /releases and source code/i });
    expect(link).toHaveAttribute("target", "_blank");
    expect(link).toHaveAttribute("rel", "noopener noreferrer");
    expect(link).toHaveAttribute("href", "https://github.com/coatyl/lantern/releases");
  });

  it("shows 'unsigned' for an unsigned build", async () => {
    vi.mocked(ipc.getBuildInfo).mockResolvedValue({ ...sample, signed: false });
    renderPane();
    expect(await screen.findByLabelText(/unsigned build/i)).toBeInTheDocument();
  });

  it("shows '✓ signed' for a signed build", async () => {
    vi.mocked(ipc.getBuildInfo).mockResolvedValue({ ...sample, signed: true });
    renderPane();
    expect(
      await screen.findByLabelText(/authenticode-signed build/i),
    ).toBeInTheDocument();
  });
});
