/**
 * Command palette data + filter helpers.
 *
 * The matching is intentionally tiny: case-insensitive substring first,
 * then a subsequence (every query character appears in order).  That is
 * enough for a nine-command list without pulling in a fuzzy library.
 */

export type CommandId =
  | "open_file"
  | "settings"
  | "save_copy"
  | "save_copy_as"
  | "run_pass"
  | "search_focus"
  | "compare_tabs"
  | "check_dead_links"
  | "merge_documents"
  | "toggle_theme";

export interface PaletteCommand {
  id: CommandId;
  label: string;
  shortcut?: string;
  enabled: boolean;
}

/** Custom events used to reach panes that own their own local state. */
export const RUN_PASS_EVENT = "lantern:run-pass";
export const SEARCH_FOCUS_EVENT = "lantern:focus-search";

export function requestRunPass(): void {
  window.dispatchEvent(new Event(RUN_PASS_EVENT));
}

export function requestSearchFocus(): void {
  window.dispatchEvent(new Event(SEARCH_FOCUS_EVENT));
}

/** Case-insensitive substring, then subsequence (fuzzy) match. */
export function matchesCommand(label: string, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (q.length === 0) return true;
  const hay = label.toLowerCase();
  if (hay.includes(q)) return true;
  let qi = 0;
  for (let i = 0; i < hay.length && qi < q.length; i++) {
    if (hay[i] === q[qi]) qi += 1;
  }
  return qi === q.length;
}

export function filterCommands<T extends { label: string }>(
  commands: T[],
  query: string,
): T[] {
  return commands.filter((c) => matchesCommand(c.label, query));
}

/** Wrap-around step used by ArrowUp / ArrowDown. */
export function stepActiveIndex(
  current: number,
  delta: number,
  length: number,
): number {
  if (length <= 0) return 0;
  return (current + delta + length) % length;
}
