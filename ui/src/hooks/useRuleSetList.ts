/**
 * Fetches the rule-set list from the backend and keeps it in sync.
 *
 * Returns a graceful fallback of the shipped built-in names when the backend is
 * unavailable (e.g. in browser-mode stubs) so the picker never comes up empty.
 */

import { useState, useEffect, useCallback } from "react";
import { ipc } from "../ipc";
import type { RuleSetSummary } from "../ipc/types";

// ---------------------------------------------------------------------------
// Fallback: mirrors the seeded built-ins so the picker works offline
// ---------------------------------------------------------------------------

const FALLBACK: RuleSetSummary[] = [
  { name: "Minimal clean",    treatment_count: 7,  is_builtin: true, path: "" },
  { name: "Aggressive scrub", treatment_count: 13, is_builtin: true, path: "" },
  { name: "Full scrub",       treatment_count: 18, is_builtin: true, path: "" },
  { name: "Find duplicates",  treatment_count: 1,  is_builtin: true, path: "" },
];

// Short descriptions for built-ins (not stored on disk, display only).
export const BUILTIN_DESCRIPTIONS: Record<string, string> = {
  "Minimal clean":
    "UTM + click-ID query params · tracking fragments · whitespace · HTML entities (titles + folders)",
  "Aggressive scrub":
    "All tracking params (UTM, click IDs, session, affiliate, search noise) · all fragments · email + handle title cleaners · folder mirrors",
  "Full scrub":
    "Everything in Aggressive scrub + user-segment paths · HTTP → HTTPS · demobilize hosts · author-suffix titles (each change requires review)",
  "Find duplicates":
    "Propose deleting bookmarks that point at the same page (ignoring http/https, www., #fragments, tracking parameters and parameter order), keeping the oldest. Review every deletion; the GUI never auto-applies.",
};

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export interface UseRuleSetListResult {
  summaries: RuleSetSummary[];
  loading: boolean;
  refresh: () => Promise<void>;
}

export function useRuleSetList(): UseRuleSetListResult {
  const [summaries, setSummaries] = useState<RuleSetSummary[]>(FALLBACK);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const list = await ipc.listRuleSets();
      if (list.length > 0) setSummaries(list);
    } catch {
      // Keep whichever summaries we have (fallback or last successful fetch).
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { summaries, loading, refresh };
}
