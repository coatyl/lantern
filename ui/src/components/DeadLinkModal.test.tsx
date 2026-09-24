import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { DeadLinkModal } from "./DeadLinkModal";
import { I18nProvider } from "../i18n/I18nProvider";
import { linkEntry, linkReport } from "../test/fixtures";
import { ipc } from "../ipc";

// Stub the IPC layer so the modal can run a check without a live backend.
vi.mock("../ipc", () => ({
  ipc: {
    checkDeadLinks: vi.fn(),
    deleteNode: vi.fn(),
  },
}));

// Stub the documents store; the modal uses refreshTree/refreshList after
// a bulk-delete to bring the rest of the UI back in sync.
vi.mock("../state/documents", () => ({
  useDocuments: (selector: (state: { refreshTree: () => Promise<void>; refreshList: () => Promise<void> }) => unknown) =>
    selector({
      refreshTree: vi.fn().mockResolvedValue(undefined),
      refreshList: vi.fn().mockResolvedValue(undefined),
    }),
}));

const ok    = (id: number, title: string)  => linkEntry({ node_id: id, title, status: { kind: "ok", code: 200 } });
const fourx = (id: number, title: string)  => linkEntry({ node_id: id, title, status: { kind: "client_error", code: 404 } });
const fivex = (id: number, title: string)  => linkEntry({ node_id: id, title, status: { kind: "server_error", code: 503 } });

const writeText = vi.fn().mockResolvedValue(undefined);

beforeEach(() => {
  vi.mocked(ipc.checkDeadLinks).mockReset();
  vi.mocked(ipc.deleteNode).mockReset();
  vi.mocked(ipc.deleteNode).mockResolvedValue(undefined);
  writeText.mockClear();
});

async function renderAndWaitForReport(report: ReturnType<typeof linkReport>) {
  vi.mocked(ipc.checkDeadLinks).mockResolvedValue(report);
  render(
    <I18nProvider locale="en">
      <DeadLinkModal
        open
        tabId={1}
        tabTitle="bookmarks.html"
        onClose={() => {}}
        onOpenSettings={() => {}}
      />
    </I18nProvider>,
  );
  // The modal kicks off a check on mount; wait for the results.
  await screen.findByText(/showing /i);
}

describe("DeadLinkModal", () => {
  it("runs a check on open and toggles a status filter from its summary card", async () => {
    const user = userEvent.setup();
    await renderAndWaitForReport(
      linkReport([ok(1, "alpha"), fourx(2, "bravo"), fivex(3, "charlie")]),
    );
    expect(screen.getByText("alpha")).toBeInTheDocument();
    expect(screen.getByText("bravo")).toBeInTheDocument();
    expect(screen.getByText("charlie")).toBeInTheDocument();

    const card4xx = screen.getByRole("button", { name: /4xx/i });
    await user.click(card4xx);
    expect(card4xx).toHaveAttribute("aria-pressed", "true");
    expect(screen.queryByText("alpha")).not.toBeInTheDocument();
    expect(screen.getByText("bravo")).toBeInTheDocument();
    expect(screen.queryByText("charlie")).not.toBeInTheDocument();
    expect(screen.getByText(/filter: 4xx/i)).toBeInTheDocument();

    // A second click clears the filter.
    await user.click(card4xx);
    expect(screen.getByText("alpha")).toBeInTheDocument();
    expect(screen.getByText("charlie")).toBeInTheDocument();
  });

  it("clicking the Time column header sorts by elapsed_ms", async () => {
    const user = userEvent.setup();
    // Mix in a non-OK entry so the "all-green" empty state doesn't hide the
    // sortable table.
    await renderAndWaitForReport(
      linkReport([
        linkEntry({ node_id: 1, title: "slow",  elapsed_ms: 900 }),
        linkEntry({ node_id: 2, title: "quick", elapsed_ms: 100 }),
        linkEntry({
          node_id: 3,
          title: "broken",
          elapsed_ms: 500,
          status: { kind: "client_error", code: 404 },
        }),
      ]),
    );
    // Two buttons in the modal contain "Time": the "Timeout" summary card
    // and the "Time" sort header.  Match the header by exact name.
    await user.click(screen.getByRole("button", { name: "Time" }));

    // We seeded three entries (quick=100, broken=500, slow=900); ascending
    // by elapsed_ms should place quick first and slow last.
    const list = screen.getByRole("list");
    const rows = within(list).getAllByRole("listitem");
    expect(rows[0]).toHaveTextContent("quick");
    expect(rows[rows.length - 1]).toHaveTextContent("slow");

    // Second click reverses.
    await user.click(screen.getByRole("button", { name: "Time" }));
    const rows2 = within(screen.getByRole("list")).getAllByRole("listitem");
    expect(rows2[0]).toHaveTextContent("slow");
    expect(rows2[rows2.length - 1]).toHaveTextContent("quick");
  });

  it("selecting rows and confirming bulk-delete invokes deleteNode for each", async () => {
    const user = userEvent.setup();
    await renderAndWaitForReport(
      linkReport([fourx(11, "broken-1"), fourx(12, "broken-2"), ok(13, "fine")]),
    );

    // Select two of the three rows (first and second checkboxes inside the list).
    const list = screen.getByRole("list");
    const itemCheckboxes = within(list).getAllByRole("checkbox");
    expect(itemCheckboxes).toHaveLength(3);
    await user.click(itemCheckboxes[0]);
    await user.click(itemCheckboxes[1]);

    // The delete button is gated by the selection count.
    const deleteBtn = screen.getByRole("button", { name: /delete 2/i });
    await user.click(deleteBtn);

    // Confirm: strip shows a separate "Delete 2" button.
    const confirmBtn = await screen.findByRole("button", { name: /^delete 2$/i });
    // Mock checkDeadLinks for the post-delete re-run.
    vi.mocked(ipc.checkDeadLinks).mockResolvedValueOnce(linkReport([ok(13, "fine")]));
    await user.click(confirmBtn);

    expect(ipc.deleteNode).toHaveBeenCalledTimes(2);
    // Both selected node ids were passed (order-independent).
    const calls = vi.mocked(ipc.deleteNode).mock.calls.map((c) => c[1]);
    expect(calls).toEqual(expect.arrayContaining([11, 12]));
  });

  it("Copy URLs writes the visible entries' URLs to the clipboard", async () => {
    // user-event v14 installs its own clipboard emulator on `setup()`, so
    // the spy has to be stamped onto `navigator` after it.
    const user = userEvent.setup();
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    await renderAndWaitForReport(
      linkReport([
        linkEntry({ node_id: 1, url: "https://a.example",  title: "a" }),
        linkEntry({ node_id: 2, url: "https://b.example",  title: "b" }),
      ]),
    );
    await user.click(screen.getByRole("button", { name: /copy urls/i }));

    expect(writeText).toHaveBeenCalledWith(
      "https://a.example\nhttps://b.example",
    );
    await waitFor(() => expect(screen.getByText(/copied/i)).toBeInTheDocument());
  });
});
