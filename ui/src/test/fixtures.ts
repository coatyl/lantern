import type {
  ChangeEntry,
  ChangeSetPreview,
  DiffSpan,
  FolderItem,
  LinkCheckEntry,
  LinkCheckReport,
  LinkStatus,
} from "../ipc/types";

export function span(tag: DiffSpan["tag"], text: string): DiffSpan {
  return { tag, text };
}

export function change(overrides: Partial<ChangeEntry> = {}): ChangeEntry {
  return {
    index: 0,
    node_id: 1,
    field: "url",
    before: "https://example.com?utm_source=x",
    after: "https://example.com",
    before_spans: [
      span("equal", "https://example.com"),
      span("removed", "?utm_source=x"),
    ],
    after_spans: [span("equal", "https://example.com")],
    treatment_id: "url.qp.utm",
    rationale: "Strip UTM parameters",
    destructive: false,
    approved: true,
    node_title: "Example",
    node_url: "https://example.com?utm_source=x",
    location: ["Reading"],
    ...overrides,
  };
}

export function preview(overrides: Partial<ChangeSetPreview> = {}): ChangeSetPreview {
  return {
    changeset_id: 1,
    rule_set_name: "Aggressive scrub",
    changes: [change()],
    ...overrides,
  };
}

export function folderItem(overrides: Partial<FolderItem> = {}): FolderItem {
  return {
    id: 1,
    kind: "bookmark",
    title: "Example",
    url: "https://example.com",
    domain: "example.com",
    add_date: 1_700_000_000,
    last_modified: 1_700_000_000,
    ...overrides,
  };
}

export function linkEntry(
  overrides: Partial<LinkCheckEntry> = {},
): LinkCheckEntry {
  return {
    node_id: 1,
    url: "https://example.com",
    title: "Example",
    status: { kind: "ok", code: 200 } as LinkStatus,
    elapsed_ms: 100,
    ...overrides,
  };
}

export function linkReport(
  entries: LinkCheckEntry[] = [],
  overrides: Partial<LinkCheckReport> = {},
): LinkCheckReport {
  return {
    entries,
    total_bookmarks: entries.length,
    probed: entries.length,
    ...overrides,
  };
}
