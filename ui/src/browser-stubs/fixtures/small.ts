/**
 * Tiny synthetic document used by Playwright E2E specs.
 *
 * Roughly ten bookmarks split across three folders, plus enough scaffolding
 * to satisfy the IPC surface the React app calls during the user stories
 * exercised in `ui/e2e/`.  Not a serious model; see `tauri-core.ts` for
 * the entry point and rationale.
 */

import type {
  AppSettings,
  BuildInfo,
  ChangeSetPreview,
  FolderItem,
  ItemPage,
  RuleSetDetail,
  RuleSetSummary,
  SearchResults,
  ShortcutBinding,
  TabInfo,
  TreatmentInfo,
  TreeNodeLazy,
  TreeView,
} from "../../ipc/types";

export interface FixtureState {
  settings: AppSettings;
  doc: { path: string };
  recentFiles: string[];
  ruleSets: RuleSetSummary[];
  treatments: TreatmentInfo[];
  shortcuts: ShortcutBinding[];
  buildInfo: BuildInfo;

  openTab: (path: string) => number;
  closeTab: (id: number) => void;
  listTabs: () => TabInfo[];
  pushRecent: (path: string) => void;
  getTree: (tab: number) => TreeView;
  getTreeRoot: (tab: number) => TreeNodeLazy[];
  getTreeChildren: (tab: number, parentId: number) => TreeNodeLazy[];
  getFolderItems: (tab: number, folderId: number) => ItemPage;
  search: (tab: number, query: Record<string, unknown>) => SearchResults;
  runPass: (tab: number, ruleSetName: string, scope?: number | null) => ChangeSetPreview;
  applyChangeset: (
    changesetId: number,
    approvals: boolean[],
  ) => { applied_count: number; skipped_count: number };
  getRuleSet: (name: string) => RuleSetDetail;
}

interface BookmarkSeed {
  id: number;
  title: string;
  url: string;
  domain: string;
}

interface FolderSeed {
  id: number;
  name: string;
  bookmarks: BookmarkSeed[];
}

const FOLDERS: FolderSeed[] = [
  {
    id: 100,
    name: "News",
    bookmarks: [
      { id: 101, title: "Hacker News", url: "https://news.ycombinator.com/", domain: "ycombinator.com" },
      { id: 102, title: "Lobsters",    url: "https://lobste.rs/",             domain: "lobste.rs" },
      { id: 103, title: "Ars Technica", url: "https://arstechnica.com/",       domain: "arstechnica.com" },
    ],
  },
  {
    id: 200,
    name: "Reference",
    bookmarks: [
      { id: 201, title: "MDN Web Docs", url: "https://developer.mozilla.org/", domain: "mozilla.org" },
      { id: 202, title: "Rust Book",    url: "https://doc.rust-lang.org/book/", domain: "rust-lang.org" },
      { id: 203, title: "TypeScript Handbook", url: "https://www.typescriptlang.org/docs/", domain: "typescriptlang.org" },
      { id: 204, title: "Tauri Docs",   url: "https://v2.tauri.app/",          domain: "tauri.app" },
    ],
  },
  {
    id: 300,
    name: "Tools",
    bookmarks: [
      { id: 301, title: "GitHub",       url: "https://github.com/",            domain: "github.com" },
      { id: 302, title: "Lantern repo", url: "https://github.com/example/lantern?utm_source=test", domain: "github.com" },
      { id: 303, title: "crates.io",    url: "https://crates.io/",             domain: "crates.io" },
    ],
  },
];

function bookmarkItem(b: BookmarkSeed): FolderItem {
  return {
    id: b.id,
    kind: "bookmark",
    title: b.title,
    url: b.url,
    domain: b.domain,
    add_date: 1_700_000_000,
    last_modified: 1_700_000_000,
  };
}

function folderItem(f: FolderSeed): FolderItem {
  return {
    id: f.id,
    kind: "folder",
    title: f.name,
    url: null,
    domain: null,
    add_date: 1_700_000_000,
    last_modified: 1_700_000_000,
  };
}

export function smallFixture(): FixtureState {
  let nextTabId = 1;
  const tabs = new Map<number, TabInfo>();

  const settings: AppSettings = {
    theme: "system",
    dead_link_checker_opt_in: false,
    recent_files_max: 10,
    crash_recovery_enabled: true,
    list_density: "compact",
    settings_path: "C:/lantern/test/settings.toml",
    rules_dir: "C:/lantern/test/rules",
  };

  const ruleSets: RuleSetSummary[] = [
    { name: "Minimal clean", treatment_count: 2, is_builtin: true, path: "C:/lantern/test/rules/minimal.toml" },
    { name: "Aggressive scrub", treatment_count: 5, is_builtin: true, path: "C:/lantern/test/rules/aggressive.toml" },
  ];

  const treatments: TreatmentInfo[] = [
    { id: "strip-utm", name: "Strip UTM tracking parameters", category: "tracking", destructive: false },
    { id: "trim-title", name: "Trim whitespace in title", category: "title", destructive: false },
  ];

  const shortcuts: ShortcutBinding[] = [
    { action_id: "open_file",  label: "Open file",  key_combo: "Ctrl+O", category: "File" },
    { action_id: "save",       label: "Save",       key_combo: "Ctrl+S", category: "File" },
    { action_id: "settings",   label: "Settings",   key_combo: "Ctrl+,", category: "App" },
    { action_id: "command_palette", label: "Command palette", key_combo: "Ctrl+K / ⌘K", category: "Tools" },
    { action_id: "compare",    label: "Compare tabs", key_combo: "Ctrl+Shift+D", category: "Tools" },
  ];

  const buildInfo: BuildInfo = {
    version: "0.2.0",
    build_flavor: "default",
    rust_version: "1.80.0",
    git_commit: null,
    license: "Apache-2.0 OR MIT",
    signed: false,
  };

  const openTab = (path: string): number => {
    const id = nextTabId++;
    tabs.set(id, {
      id,
      title: pathFilename(path) || "fixture.html",
      path,
      dirty: false,
      stats: {
        bookmark_count: FOLDERS.reduce((n, f) => n + f.bookmarks.length, 0),
        folder_count: FOLDERS.length,
        separator_count: 0,
      },
    });
    return id;
  };

  // Seed a small recent-files collection so the library home can render
  // a populated stack in fixture mode.  `list_recent_files` used to
  // return [] which made the home look empty even after files were
  // opened.
  const recentFiles = [
    "C:/lantern/test/fixtures/small.html",
    "C:/Users/fixture/Documents/bookmarks-firefox.html",
    "C:/Users/fixture/Downloads/chrome-bookmarks.html",
  ];

  const pushRecent = (path: string) => {
    const next = recentFiles.filter((entry) => entry !== path);
    next.unshift(path);
    recentFiles.splice(0, recentFiles.length, ...next.slice(0, settings.recent_files_max));
  };

  // Pre-open one tab so the library home is bypassed in fixture mode and
  // specs can drive the three-pane workspace without first having to open
  // a file.  The first call to `list_tabs` already returns this row.
  openTab("C:/lantern/test/fixtures/small.html");

  return {
    settings,
    doc: { path: "C:/lantern/test/fixtures/small.html" },
    recentFiles,
    ruleSets,
    treatments,
    shortcuts,
    buildInfo,

    openTab,
    pushRecent,
    closeTab(id) {
      tabs.delete(id);
    },
    listTabs() {
      return [...tabs.values()];
    },

    getTree(_tab) {
      return {
        root: {
          id: 0,
          name: "Bookmarks",
          children: FOLDERS.map((f) => ({
            id: f.id,
            name: f.name,
            children: [],
          })),
        },
      };
    },
    getTreeRoot(_tab) {
      return FOLDERS.map((f) => ({
        id: f.id,
        name: f.name,
        has_children: false,
      }));
    },
    getTreeChildren(_tab, _parentId) {
      return [];
    },

    getFolderItems(_tab, folderId) {
      if (folderId === 0) {
        const items = FOLDERS.map(folderItem);
        return { items, total: items.length };
      }
      const folder = FOLDERS.find((f) => f.id === folderId);
      if (!folder) return { items: [], total: 0 };
      const items = folder.bookmarks.map(bookmarkItem);
      return { items, total: items.length };
    },

    search(_tab, query) {
      const q = String(query.query ?? "").toLowerCase();
      const titles = Boolean(query.search_titles ?? true);
      const urls = Boolean(query.search_urls ?? true);
      const mode = String(query.mode ?? "substring");
      // Mirrors the backend: an invalid pattern is an error, not "no results".
      const pattern =
        mode === "regex"
          ? new RegExp(q, "i")
          : mode === "glob"
            ? new RegExp(q.replace(/[.+^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*").replace(/\?/g, "."), "i")
            : null;
      const matches = (text: string) => (pattern ? pattern.test(text) : text.toLowerCase().includes(q));
      const items: FolderItem[] = [];
      for (const folder of FOLDERS) {
        for (const b of folder.bookmarks) {
          const matchTitle = titles && matches(b.title);
          const matchUrl = urls && matches(b.url);
          if (q.length > 0 && (matchTitle || matchUrl)) {
            items.push(bookmarkItem(b));
          }
        }
      }
      return { items, total: items.length };
    },

    runPass(_tab, ruleSetName) {
      // A little of everything the review surface has to render: a tracking
      // parameter, a title fix, a folder rename, and a duplicate deletion
      // (destructive, so it starts unapproved).
      const eq = (text: string) => ({ tag: "equal" as const, text });
      const rm = (text: string) => ({ tag: "removed" as const, text });
      const add = (text: string) => ({ tag: "added" as const, text });
      const preview: ChangeSetPreview = {
        changeset_id: 1,
        rule_set_name: ruleSetName,
        changes: [
          {
            index: 0,
            node_id: 302,
            field: "url",
            before: "https://github.com/example/lantern?utm_source=test",
            after: "https://github.com/example/lantern",
            before_spans: [eq("https://github.com/example/lantern"), rm("?utm_source=test")],
            after_spans: [eq("https://github.com/example/lantern")],
            treatment_id: "url.qp.utm",
            rationale: "Removed tracking query parameter",
            destructive: false,
            approved: true,
            node_title: "Lantern repo",
            node_url: "https://github.com/example/lantern?utm_source=test",
            location: ["Tools"],
          },
          {
            index: 1,
            node_id: 203,
            field: "title",
            before: "TypeScript Handbook  | TypeScript",
            after: "TypeScript Handbook",
            before_spans: [eq("TypeScript Handbook"), rm("  | TypeScript")],
            after_spans: [eq("TypeScript Handbook")],
            treatment_id: "title.site_suffix",
            rationale: "Removed site-name suffix",
            destructive: false,
            approved: true,
            node_title: "TypeScript Handbook  | TypeScript",
            node_url: "https://www.typescriptlang.org/docs/",
            location: ["Reference"],
          },
          {
            index: 2,
            node_id: 300,
            field: "folder_name",
            before: "Tools ",
            after: "Tools",
            before_spans: [eq("Tools"), rm(" ")],
            after_spans: [eq("Tools"), add("")],
            treatment_id: "folder.whitespace",
            rationale: "Trimmed whitespace",
            destructive: false,
            approved: true,
            node_title: "Tools ",
            node_url: null,
            location: [],
          },
          {
            index: 3,
            node_id: 204,
            field: "node",
            before: "",
            after: "(deleted)",
            before_spans: [],
            after_spans: [],
            treatment_id: "structure.duplicates.exact_url",
            rationale: "Exact duplicate of \u201cGitHub\u201d in Tools",
            destructive: true,
            approved: false,
            node_title: "GitHub (saved twice)",
            node_url: "https://github.com/",
            location: ["Reference"],
          },
        ],
      };
      return preview;
    },

    applyChangeset(_changesetId, approvals) {
      const applied = approvals.filter(Boolean).length;
      const skipped = approvals.length - applied;
      return { applied_count: applied, skipped_count: skipped };
    },

    getRuleSet(name) {
      return {
        name,
        treatment_ids: treatments.map((t) => t.id),
        treatments: treatments.map((t) => ({ id: t.id, config: null })),
        is_builtin: true,
        path: "C:/lantern/test/rules/" + name + ".toml",
      };
    },
  };

  function pathFilename(p: string): string {
    const idx = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
    return idx >= 0 ? p.slice(idx + 1) : p;
  }
}
