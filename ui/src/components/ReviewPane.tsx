/**
 * Review surface for a proposed change set.
 *
 * Replaces the list + detail panes while a pass is awaiting a decision, so
 * the review gets the width it needs: full before/after URLs, one card per
 * touched bookmark or folder (with its folder path), kind filters, and a
 * single Apply action whose label says exactly what will happen.
 *
 * Nothing is applied until the user presses Apply; Discard (or Escape)
 * drops the change set.  A successful apply raises a toast with Undo.
 */

import { useEffect, useMemo, useRef, useState } from "react";

import { ipc } from "../ipc";
import { useDocuments, type PendingReview } from "../state/documents";
import { useToast } from "../hooks/useToast";
import { useT } from "../i18n/I18nProvider";
import type { ChangeEntry } from "../ipc/types";
import { DiffLine } from "./DiffLine";
import { FolderIcon, LinkIcon, SearchIcon } from "./Icons";

export type ChangeKindFilter = "all" | "url" | "title" | "folder_name" | "node";

const KIND_ORDER: Exclude<ChangeKindFilter, "all">[] = ["url", "title", "folder_name", "node"];

/** Filter bucket for a change; anything unexpected (e.g. flags) counts as URL-level metadata. */
export function kindOf(change: ChangeEntry): Exclude<ChangeKindFilter, "all"> {
  switch (change.field) {
    case "title":
    case "folder_name":
    case "node":
      return change.field;
    default:
      return "url";
  }
}

interface NodeGroup {
  nodeId: number;
  title: string;
  url: string | null;
  location: string[];
  isFolder: boolean;
  changes: ChangeEntry[];
}

/** Group changes by the node they touch, keeping first-seen order. */
export function groupByNode(changes: ChangeEntry[]): NodeGroup[] {
  const groups = new Map<number, NodeGroup>();
  for (const change of changes) {
    let group = groups.get(change.node_id);
    if (!group) {
      group = {
        nodeId: change.node_id,
        title: change.node_title,
        url: change.node_url,
        location: change.location,
        isFolder: change.node_url === null && change.field !== "url",
        changes: [],
      };
      groups.set(change.node_id, group);
    }
    group.changes.push(change);
  }
  return [...groups.values()];
}

function matchesQuery(group: NodeGroup, change: ChangeEntry, q: string): boolean {
  if (!q) return true;
  const hay = [group.title, group.url ?? "", change.before, change.after, change.rationale, ...group.location];
  return hay.some((s) => s.toLowerCase().includes(q));
}

export interface ReviewPaneProps {
  review: PendingReview;
}

export function ReviewPane({ review }: ReviewPaneProps) {
  const t = useT();
  const { toast } = useToast();
  const { refreshTabs, refreshTree, refreshList, closeReview } = useDocuments();
  const { preview, tabId, scopeLabel } = review;

  const [approvals, setApprovals] = useState<boolean[]>(() => preview.changes.map((c) => c.approved));
  const [kind, setKind] = useState<ChangeKindFilter>("all");
  const [query, setQuery] = useState("");
  const [applying, setApplying] = useState(false);
  const headingRef = useRef<HTMLHeadingElement>(null);

  // A new change set (e.g. the user re-ran the pass) resets every decision.
  useEffect(() => {
    setApprovals(preview.changes.map((c) => c.approved));
    setKind("all");
    setQuery("");
    headingRef.current?.focus();
  }, [preview]);

  const groups = useMemo(() => groupByNode(preview.changes), [preview]);

  const counts = useMemo(() => {
    const c: Record<Exclude<ChangeKindFilter, "all">, number> = { url: 0, title: 0, folder_name: 0, node: 0 };
    for (const change of preview.changes) c[kindOf(change)] += 1;
    return c;
  }, [preview]);

  const q = query.trim().toLowerCase();
  const visibleGroups = useMemo(
    () =>
      groups
        .map((g) => ({
          ...g,
          changes: g.changes.filter(
            (c) => (kind === "all" || kindOf(c) === kind) && matchesQuery(g, c, q),
          ),
        }))
        .filter((g) => g.changes.length > 0),
    [groups, kind, q],
  );
  const visibleIndexes = visibleGroups.flatMap((g) => g.changes.map((c) => c.index));

  const selected = approvals.filter(Boolean).length;
  const selectedDeletes = preview.changes.filter((c, i) => approvals[i] && c.field === "node").length;
  const total = preview.changes.length;

  const setMany = (indexes: number[], value: boolean) =>
    setApprovals((prev) => {
      const next = [...prev];
      for (const i of indexes) next[i] = value;
      return next;
    });

  const discard = () => closeReview();

  const apply = async () => {
    if (applying || selected === 0) return;
    setApplying(true);
    try {
      const report = await ipc.applyChangeset(tabId, preview.changeset_id, approvals);
      closeReview();
      await Promise.all([refreshTabs(), refreshTree(), refreshList()]);
      toast(t("review.applied", { n: report.applied_count }), "success", {
        label: t("review.undo"),
        onClick: async () => {
          try {
            await ipc.undo(tabId);
            await Promise.all([refreshTabs(), refreshTree(), refreshList()]);
          } catch (e) {
            toast(String(e), "error");
          }
        },
      });
    } catch (e) {
      // Keep the review open so the user can adjust and retry.
      toast(String(e), "error");
      setApplying(false);
    }
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      void apply();
      return;
    }
    if (e.key === "Escape") {
      const inFilter = (e.target as HTMLElement).closest("[data-review-filter]");
      if (inFilter && query) return; // the input clears itself first
      e.preventDefault();
      discard();
    }
  };

  const applyLabel = applying
    ? t("review.applying")
    : selectedDeletes > 0
      ? t("review.applyWithDeletes", { n: selected, deletions: t("review.deletions", { n: selectedDeletes }) })
      : t("review.apply", { n: selected });

  return (
    <section
      aria-label={t("review.region")}
      onKeyDown={onKeyDown}
      className="flex-1 min-w-0 flex flex-col bg-surface-0 animate-fade-in"
    >
      {/* ── Header ─────────────────────────────────────────────────────── */}
      <header className="flex items-start justify-between gap-4 px-5 pt-4 pb-3 border-b">
        <div className="min-w-0">
          <h2
            ref={headingRef}
            tabIndex={-1}
            className="text-base font-semibold text-neutral-100 focus:outline-none"
          >
            {t("review.title")}
          </h2>
          <p className="mt-0.5 text-xs text-neutral-500 truncate">
            {t("review.meta", { ruleSet: preview.rule_set_name, scope: scopeLabel, n: total })}
          </p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <span className="text-xs text-neutral-500 tabular-nums mr-1" aria-live="polite">
            {t("review.selected", { n: selected, total })}
          </span>
          <button
            type="button"
            onClick={discard}
            disabled={applying}
            className="px-3 py-1.5 rounded-md border border-neutral-700 text-xs text-neutral-300
                       hover:text-neutral-100 hover:border-neutral-500 transition-colors
                       disabled:opacity-40"
          >
            {t("review.discard")}
          </button>
          <button
            type="button"
            onClick={() => void apply()}
            disabled={applying || selected === 0}
            title={t("review.applyHint")}
            className={`px-4 py-1.5 rounded-md text-xs font-semibold transition-colors
                        disabled:opacity-40 ${
                          selectedDeletes > 0
                            ? "bg-danger hover:bg-danger/85 text-on-danger"
                            : "bg-accent hover:bg-accent-hover text-on-accent"
                        }`}
          >
            {applyLabel}
          </button>
        </div>
      </header>

      {/* ── Filters ────────────────────────────────────────────────────── */}
      <div className="flex flex-wrap items-center gap-2 px-5 py-2 border-b">
        <div role="group" aria-label={t("review.kindFilter")} className="flex flex-wrap gap-1">
          <KindChip label={t("review.kind.all")} count={total} active={kind === "all"} onClick={() => setKind("all")} />
          {KIND_ORDER.filter((k) => counts[k] > 0).map((k) => (
            <KindChip
              key={k}
              label={t(`review.kind.${k}`)}
              count={counts[k]}
              active={kind === k}
              danger={k === "node"}
              onClick={() => setKind(k)}
            />
          ))}
        </div>

        <div data-review-filter className="relative ml-auto w-56 max-w-full">
          <SearchIcon className="w-3.5 h-3.5 absolute left-2 top-1/2 -translate-y-1/2 text-neutral-600" />
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape" && query) {
                e.stopPropagation();
                setQuery("");
              }
            }}
            placeholder={t("review.filterPlaceholder")}
            aria-label={t("review.filterPlaceholder")}
            className="w-full bg-surface-2 border border-neutral-800 rounded-md pl-7 pr-2 py-1
                       text-xs text-neutral-200 placeholder-neutral-600
                       focus:outline-none focus:border-accent/60"
          />
        </div>

        <div className="flex items-center gap-1 text-xs">
          <button
            type="button"
            onClick={() => setMany(visibleIndexes, true)}
            className="px-2 py-1 rounded text-neutral-400 hover:text-neutral-100 hover:bg-surface-2"
          >
            {kind === "all" && !q ? t("review.selectAll") : t("review.selectShown")}
          </button>
          <button
            type="button"
            onClick={() => setMany(visibleIndexes, false)}
            className="px-2 py-1 rounded text-neutral-400 hover:text-neutral-100 hover:bg-surface-2"
          >
            {kind === "all" && !q ? t("review.clearAll") : t("review.clearShown")}
          </button>
        </div>
      </div>

      {/* ── Changes ────────────────────────────────────────────────────── */}
      <div className="flex-1 min-h-0 overflow-y-auto px-5 py-4" aria-busy={applying}>
        {visibleGroups.length === 0 ? (
          <p className="text-sm text-neutral-500 text-center py-12">{t("review.noMatches")}</p>
        ) : (
          <ul className="flex flex-col gap-3 max-w-5xl mx-auto">
            {visibleGroups.map((group) => (
              <GroupCard
                key={group.nodeId}
                group={group}
                approvals={approvals}
                onToggle={(i) => setMany([i], !approvals[i])}
              />
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}

// ---------------------------------------------------------------------------

function KindChip({
  label,
  count,
  active,
  danger,
  onClick,
}: {
  label: string;
  count: number;
  active: boolean;
  danger?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onClick}
      className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full border text-xs transition-colors ${
        active
          ? danger
            ? "border-danger/60 bg-danger/15 text-danger"
            : "border-accent/60 bg-accent/15 text-accent"
          : "border-neutral-800 text-neutral-400 hover:text-neutral-200 hover:border-neutral-700"
      }`}
    >
      {label}
      <span className="tabular-nums text-[11px] opacity-80">{count}</span>
    </button>
  );
}

function GroupCard({
  group,
  approvals,
  onToggle,
}: {
  group: NodeGroup;
  approvals: boolean[];
  onToggle: (index: number) => void;
}) {
  const t = useT();
  return (
    <li className="rounded-lg border border-neutral-800 bg-surface-1 overflow-hidden">
      <div className="flex items-center gap-2 px-3 py-2 border-b border-neutral-800/70 bg-surface-2/40">
        <span className="text-accent shrink-0" aria-hidden>
          {group.isFolder ? <FolderIcon className="w-3.5 h-3.5" /> : <LinkIcon className="w-3.5 h-3.5" />}
        </span>
        <span className="text-sm font-medium text-neutral-100 truncate">
          {group.title || <em className="not-italic text-neutral-500">{t("review.untitled")}</em>}
        </span>
        {group.location.length > 0 && (
          <span className="ml-auto text-[11px] text-neutral-500 truncate max-w-[45%]" title={group.location.join(" › ")}>
            {group.location.join(" › ")}
          </span>
        )}
      </div>
      <ul className="divide-y divide-neutral-800/60">
        {group.changes.map((change) => (
          <ChangeLine
            key={change.index}
            change={change}
            nodeUrl={group.url}
            approved={approvals[change.index]}
            onToggle={() => onToggle(change.index)}
          />
        ))}
      </ul>
    </li>
  );
}

function ChangeLine({
  change,
  nodeUrl,
  approved,
  onToggle,
}: {
  change: ChangeEntry;
  nodeUrl: string | null;
  approved: boolean;
  onToggle: () => void;
}) {
  const t = useT();
  const k = kindOf(change);
  const isDelete = k === "node";
  return (
    <li>
      <label
        className={`flex items-start gap-3 px-3 py-2.5 cursor-pointer transition-colors hover:bg-surface-2/60 ${
          approved ? "" : "opacity-70"
        }`}
      >
        <input
          type="checkbox"
          checked={approved}
          onChange={onToggle}
          aria-label={`${t(`review.kind.${k}`)}: ${change.rationale}`}
          className="mt-0.5 shrink-0"
        />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <span
              className={`px-1.5 py-px rounded text-[10px] font-semibold uppercase tracking-wider ${
                isDelete ? "bg-danger/15 text-danger" : "bg-surface-3 text-neutral-400"
              }`}
            >
              {t(`review.kind.${k}`)}
            </span>
            <span className="text-xs text-neutral-400 truncate">{change.rationale}</span>
            {change.destructive && !isDelete && (
              <span className="px-1.5 py-px rounded bg-danger/15 text-danger text-[10px]">
                {t("review.destructive")}
              </span>
            )}
          </div>
          {isDelete ? (
            <p className="text-xs text-neutral-400 break-all">
              {t("review.deleteNode")}
              {nodeUrl && <span className="ml-1 font-mono text-[11px] text-neutral-500">{nodeUrl}</span>}
            </p>
          ) : (
            <div className="font-mono space-y-0.5">
              <DiffLine spans={change.before_spans} fallback={change.before} side="before" wrap />
              <DiffLine spans={change.after_spans} fallback={change.after} side="after" wrap />
            </div>
          )}
        </div>
      </label>
    </li>
  );
}
