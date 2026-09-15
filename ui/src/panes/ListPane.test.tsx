/**
 * ListPane: virtualisation tests (v0.0.7 perf hardening, slice 1).
 *
 * The interesting question for these tests is *not* "do all the rows
 * render correctly?" (that's covered by the rest of the suite) but
 * "does the virtualisation actually keep DOM size bounded for huge
 * folders, and does scrolling reveal new rows?".  We mount with 50 000
 * fake items and assert:
 *
 *   1. only a small window of `[data-item-row]` nodes is in the DOM,
 *   2. scrolling the viewport materially changes which row indices are
 *      mounted (a far-down row appears).
 *
 * react-window measures geometry off `getBoundingClientRect()` /
 * `clientHeight` and `scrollTop`.  jsdom returns 0 for both by default,
 * which would make the list render nothing.  We patch the prototypes
 * once at module load so the values are deterministic.
 */

import { describe, it, expect, vi, beforeAll } from "vitest";
import { act, render, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nProvider } from "../i18n/I18nProvider";
import type { FolderItem, ItemPage } from "../ipc/types";

// ─── Stub Tauri IPC (ListPane imports it but never calls it in these
//     tests since we never trigger DnD, rename, or search). ──────────
vi.mock("../ipc", () => ({
  ipc: {
    renameNode:    vi.fn(),
    moveNode:      vi.fn(),
    getFolderItems: vi.fn(),
  },
}));

// ─── Stub the documents store. ListPane reads the whole state object
//     (no selector), so we return a single fixed snapshot.  We expose
//     a mutable reference so individual tests can plug in different
//     listPage payloads. ─────────────────────────────────────────────
interface StoreSnapshot {
  listPage:        ItemPage | null;
  sortSpec:        { column: "title"; descending: false };
  isSearchMode:    boolean;
  searchResults:   null;
  activeTab:       "tab-1";
  selectedItem:    FolderItem | null;
  folderStack:     [];
  page:            number;
  pageSize:        number;
  filter:          null;
  refreshList:     () => Promise<void>;
  refreshTree:     () => Promise<void>;
  runSearch:       () => Promise<void>;
  clearSearch:     () => void;
  setSelectedItem: (item: FolderItem | null) => void;
  setPendingDelete: () => void;
  navigateTo:      () => Promise<void>;
  navigateToAncestor: () => Promise<void>;
  setPage:         () => Promise<void>;
  createBookmark:  () => Promise<number>;
  createFolder:    () => Promise<number>;
  createSeparator: () => Promise<number>;
  setFilter:       () => Promise<void>;
}

let storeState: StoreSnapshot;

function makeStore(items: FolderItem[]): StoreSnapshot {
  return {
    listPage:        { items, total: items.length },
    sortSpec:        { column: "title", descending: false },
    isSearchMode:    false,
    searchResults:   null,
    activeTab:       "tab-1",
    selectedItem:    null,
    folderStack:     [],
    page:            1,
    pageSize:        items.length || 200,
    filter:          null,
    refreshList:     vi.fn().mockResolvedValue(undefined),
    refreshTree:     vi.fn().mockResolvedValue(undefined),
    runSearch:       vi.fn().mockResolvedValue(undefined),
    clearSearch:     vi.fn(),
    setSelectedItem: vi.fn((it) => { storeState.selectedItem = it; }),
    setPendingDelete: vi.fn(),
    navigateTo:      vi.fn().mockResolvedValue(undefined),
    navigateToAncestor: vi.fn().mockResolvedValue(undefined),
    setPage:         vi.fn().mockResolvedValue(undefined),
    createBookmark:  vi.fn().mockResolvedValue(0),
    createFolder:    vi.fn().mockResolvedValue(0),
    createSeparator: vi.fn().mockResolvedValue(0),
    setFilter:       vi.fn().mockResolvedValue(undefined),
  };
}

vi.mock("../state/documents", () => {
  const useDocuments = (() => storeState) as unknown as {
    (): StoreSnapshot;
    setState: (...args: unknown[]) => void;
    getState: () => StoreSnapshot & { activeFolderId: () => number };
  };
  useDocuments.setState = vi.fn();
  useDocuments.getState = () => ({ ...storeState, activeFolderId: () => 0 });
  return {
    useDocuments,
    isFilterActive: () => false,
    PAGE_SIZE: 200,
  };
});

// Imported *after* the mocks so the module-level closure picks them up.
import ListPane from "./ListPane";

// ─── Geometry stubs.  jsdom returns 0 for layout reads; we hand
//     react-window concrete values so it can compute a visible
//     window. ────────────────────────────────────────────────────────
const VIEWPORT_HEIGHT = 600;

beforeAll(() => {
  // ResizeObserver: used by useElementSize.
  class StubResizeObserver {
    private cb: ResizeObserverCallback;
    constructor(cb: ResizeObserverCallback) { this.cb = cb; }
    observe(target: Element) {
      // Fire once synchronously so useElementSize commits a non-zero
      // height before the next render.
      this.cb(
        [{ target, contentRect: { width: 800, height: VIEWPORT_HEIGHT } } as ResizeObserverEntry],
        this as unknown as ResizeObserver,
      );
    }
    unobserve() {}
    disconnect() {}
  }
  globalThis.ResizeObserver = StubResizeObserver as unknown as typeof ResizeObserver;

  // getBoundingClientRect: used by useElementSize's first synchronous
  // measurement and by react-window internally on some code paths.
  const rectSpy = () => ({
    x: 0, y: 0, top: 0, left: 0, right: 800, bottom: VIEWPORT_HEIGHT,
    width: 800, height: VIEWPORT_HEIGHT, toJSON() { return {}; },
  });
  Element.prototype.getBoundingClientRect = rectSpy as unknown as typeof Element.prototype.getBoundingClientRect;

  // react-window reads `clientHeight` / `clientWidth` (viewport) and
  // `scrollHeight` (total content) to detect resize and clamp scrollTop.
  // jsdom returns 0 for all three; we hand it real numbers so react-window
  // can compute a window of visible rows and accept programmatic
  // `scrollTop` writes during the scrolling test.
  Object.defineProperty(HTMLElement.prototype, "clientHeight", {
    configurable: true,
    get() { return VIEWPORT_HEIGHT; },
  });
  Object.defineProperty(HTMLElement.prototype, "clientWidth", {
    configurable: true,
    get() { return 800; },
  });
  Object.defineProperty(HTMLElement.prototype, "scrollHeight", {
    configurable: true,
    // 50 000 rows × 28 px each is plenty of room: anything larger than
    // VIEWPORT_HEIGHT is fine; react-window only uses this to clamp.
    get() { return 50_000 * 28; },
  });
});

function makeBookmarks(n: number): FolderItem[] {
  const items: FolderItem[] = new Array(n);
  for (let i = 0; i < n; i++) {
    items[i] = {
      id:            i + 1,
      kind:          "bookmark",
      title:         `Bookmark ${i}`,
      url:           `https://example.com/${i}`,
      domain:        "example.com",
      add_date:      1_700_000_000 + i,
      last_modified: 1_700_000_000 + i,
    };
  }
  return items;
}

describe("ListPane inline rename (v0.0.11)", () => {
  it("double-click on a row's title cell swaps it for an input; Escape cancels", async () => {
    const items: FolderItem[] = [
      {
        id: 101,
        kind: "bookmark",
        title: "Original Title",
        url: "https://example.com/x",
        domain: "example.com",
        add_date: 1_700_000_000,
        last_modified: 1_700_000_000,
      },
    ];
    storeState = makeStore(items);

    const user = userEvent.setup();
    const { container } = render(
      <I18nProvider locale="en">
        <ListPane />
      </I18nProvider>,
    );

    // The row uses a [data-title-cell] span as the dedicated rename trigger.
    const titleCell = await waitFor(() =>
      container.querySelector<HTMLElement>('[data-title-cell]'),
    );
    expect(titleCell).not.toBeNull();
    expect(titleCell!.textContent).toBe("Original Title");

    // Double-click swaps it for the rename input.
    fireEvent.doubleClick(titleCell!);

    const input = await waitFor(() =>
      container.querySelector<HTMLInputElement>('input[data-rename-input]'),
    );
    expect(input).not.toBeNull();
    expect(input!.value).toBe("Original Title");

    // Escape cancels: input goes away, original title is preserved.
    await user.keyboard("{Escape}");

    await waitFor(() => {
      expect(
        container.querySelector('input[data-rename-input]'),
      ).toBeNull();
    });
    const restoredCell = container.querySelector<HTMLElement>('[data-title-cell]');
    expect(restoredCell?.textContent).toBe("Original Title");
  });
});

describe("ListPane virtualisation", () => {
  it("renders only the visible window of rows for a 50,000-row list", () => {
    storeState = makeStore(makeBookmarks(50_000));

    render(
      <I18nProvider locale="en">
        <ListPane />
      </I18nProvider>,
    );

    const rendered = document.querySelectorAll("[data-item-row]");
    // Only a small window's worth (viewport height / row height + a
    // generous overscan + dnd-kit's drop target), call it "a couple
    // hundred at most".  The exact number depends on overscan; we
    // assert it's nowhere near 50k.
    expect(rendered.length).toBeGreaterThan(0);
    expect(rendered.length).toBeLessThan(200);
  });

  it("scrolling reveals later rows", () => {
    storeState = makeStore(makeBookmarks(50_000));

    const { container } = render(
      <I18nProvider locale="en">
        <ListPane />
      </I18nProvider>,
    );

    // Before scrolling, "Bookmark 0" should be in the DOM and a
    // far-down item should not be.
    expect(container.textContent).toContain("Bookmark 0");
    expect(container.textContent).not.toContain("Bookmark 1000");

    // The FixedSizeList renders a scrollable div as the second child
    // of the viewport wrapper.  We find the scroller by its style
    // (`overflow: auto`) which react-window applies to the outer
    // element it owns.
    const scrollers = container.querySelectorAll<HTMLDivElement>(
      'div[style*="overflow"]',
    );
    const scroller = Array.from(scrollers).find((el) => {
      const style = el.getAttribute("style") ?? "";
      return style.includes("overflow") && el.scrollHeight !== el.clientHeight;
    }) ?? scrollers[scrollers.length - 1];
    expect(scroller).toBeDefined();

    // Scroll deep into the list (28 px rows × ~1000 = 28 000 px).
    act(() => {
      scroller.scrollTop = 28_000;
      scroller.dispatchEvent(new Event("scroll"));
    });

    // After scrolling, a far-down row's text should now be present
    // somewhere in the DOM.  We check for any "Bookmark 9##" which
    // is well within the new visible window.
    const html = container.innerHTML;
    expect(html).toMatch(/Bookmark (9\d\d|1\d{3})/);
  });
});
