/**
 * Pure helpers shared by the rule-set editor: per-treatment config handling
 * and treatment-category presentation.
 */

import type { RuleSetDetail } from "../../ipc/types";

export type ConfigValue = Record<string, unknown> | null;
export type ConfigsMap = Record<string, ConfigValue>;

/** Treatments whose TOML config block the editor can edit. */
export const CONFIGURABLE_IDS = new Set(["url.qp.custom", "title.regex", "folder.regex"]);

/** Config seeded when a configurable treatment has none yet. */
export function defaultConfigFor(id: string): ConfigValue {
  switch (id) {
    case "url.qp.custom":
      return { params: [] };
    case "title.regex":
    case "folder.regex":
      return { pattern: "", replacement: "" };
    default:
      return null;
  }
}

/**
 * The editable config map for a loaded rule set, seeding defaults for any
 * configurable treatment without a persisted config (e.g. a fresh duplicate
 * of a built-in).
 */
export function configsFromDetail(detail: RuleSetDetail): ConfigsMap {
  const out: ConfigsMap = {};
  for (const t of detail.treatments) {
    if (CONFIGURABLE_IDS.has(t.id)) out[t.id] = t.config ?? defaultConfigFor(t.id);
  }
  return out;
}

export function categoryLabel(cat: string): string {
  if (cat.startsWith("url")) return "URL";
  if (cat === "title") return "Title";
  if (cat === "folder_name") return "Folder";
  if (cat === "cross_field") return "Cross";
  return cat;
}

export function categoryColor(cat: string): string {
  if (cat.startsWith("url")) return "text-info bg-info/10";
  if (cat === "title") return "text-warn bg-warn/10";
  if (cat === "folder_name") return "text-ok bg-ok/10";
  if (cat === "cross_field") return "text-violet-400 bg-violet-400/10";
  return "text-neutral-400 bg-neutral-400/10";
}

export function treatmentCount(n: number): string {
  return `${n} treatment${n === 1 ? "" : "s"}`;
}
