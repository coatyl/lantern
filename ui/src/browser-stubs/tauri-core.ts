/**
 * Browser stub for @tauri-apps/api/core.
 *
 * Active only in plain-browser dev (Vite without the Tauri CLI). The real
 * module is used when the app runs inside a Tauri WebView.
 *
 * # Behaviour
 *
 * Two modes are supported:
 *
 * 1. **Default** (no fixture).  Every `invoke` rejects.  Matches the
 *    historical behaviour the welcome screen relies on: non-critical
 *    IPC paths swallow the rejection in their `try { ... } catch {}`
 *    blocks and render reasonable defaults.
 *
 * 2. **Fixture mode**, opted into by setting
 *    `localStorage.setItem("lantern.test.fixture", "small")` before the
 *    first React render (e.g. via `page.addInitScript` in Playwright).
 *    A small synthetic document is exposed via the same `invoke`
 *    surface the real Rust backend uses, so cross-component E2E specs
 *    can drive the UI without booting Tauri.
 *
 * The fixture is intentionally minimal: just enough to validate the
 * happy path of US-001/002/003/008/009/013/018.  Treat it as a
 * conversation prop, not a serious model.
 */

import { smallFixture, type FixtureState } from "./fixtures/small";

type InvokeArgs = Record<string, unknown> | undefined;

const STORAGE_KEY = "lantern.test.fixture";

let state: FixtureState | null = null;

function fixtureName(): string | null {
  if (typeof localStorage === "undefined") return null;
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function ensureState(): FixtureState | null {
  const name = fixtureName();
  if (!name) return null;
  if (!state) state = smallFixture();
  return state;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function invoke<T>(cmd: string, args?: InvokeArgs): Promise<T> {
  const fx = ensureState();
  if (!fx) {
    return Promise.reject(new Error(`[browser-stub] invoke("${cmd}"): no Tauri runtime`));
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const result: any = dispatch(fx, cmd, args ?? {});
  if (result === undefined) {
    return Promise.reject(
      new Error(`[browser-stub] invoke("${cmd}"): not implemented in fixture`),
    );
  }
  return Promise.resolve(result as T);
}

// ---------------------------------------------------------------------------
// Fixture command dispatcher
// ---------------------------------------------------------------------------

function dispatch(
  fx: FixtureState,
  cmd: string,
  args: Record<string, unknown>,
): unknown {
  switch (cmd) {
    case "list_recent_files":
      return [];
    case "get_recovery_state":
      return { paths: [] };
    case "clear_recent_files":
    case "dismiss_recovery_session":
      return undefined;

    case "get_settings":
      return fx.settings;
    case "update_settings": {
      const next = (args.settings ?? null) as FixtureState["settings"] | null;
      if (next) fx.settings = next;
      return undefined;
    }

    case "open_file": {
      const path = (args.path as string) ?? fx.doc.path;
      const tabId = fx.openTab(path);
      return tabId;
    }
    case "list_tabs":
      return fx.listTabs();
    case "close_tab":
      fx.closeTab(args.tab as number);
      return undefined;

    case "get_tree":
      return fx.getTree(args.tab as number);
    case "get_tree_root":
      return fx.getTreeRoot(args.tabId as number);
    case "get_tree_children":
      return fx.getTreeChildren(args.tabId as number, args.parentId as number);

    case "get_folder_items":
      return fx.getFolderItems(args.tab as number, args.folderId as number);

    case "search":
      return fx.search(args.tab as number, args.query as Record<string, unknown>);

    case "run_pass":
      return fx.runPass(args.tab as number, args.ruleSetName as string);
    case "apply_changeset":
      return fx.applyChangeset(args.changesetId as number, args.approvals as boolean[]);

    case "list_rule_sets":
      return fx.ruleSets;
    case "get_rule_set":
      return fx.getRuleSet(args.name as string);
    case "list_treatments":
      return fx.treatments;

    case "list_shortcuts":
      return fx.shortcuts;
    case "get_logs":
      return [];
    case "get_build_info":
      return fx.buildInfo;

    case "undo":
    case "redo":
    case "export":
      return undefined;
  }
  return undefined;
}
