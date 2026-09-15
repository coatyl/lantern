/**
 * Rule-set editor modal.
 *
 * Two-column layout:
 *  Left:   list of rule sets with + New / Duplicate / Delete actions.
 *  Right:  treatment drag-reorder list for the selected set; Save button.
 *
 * Built-in rule sets are read-only: treatments and name cannot be changed.
 * They can be duplicated to create an editable copy.
 */

import {
  useState,
  useEffect,
  useCallback,
  useMemo,
  useRef,
} from "react";
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
  arrayMove,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { ipc } from "../ipc";
import type {
  RuleSetDetail,
  RuleSetSummary,
  RuleSetTreatment,
  TreatmentInfo,
} from "../ipc/types";
import { useFocusTrap } from "../hooks/useFocusTrap";
import {
  PlusIcon,
  CopyIcon,
  TrashIcon,
  XIcon,
  GripIcon,
  LockIcon,
} from "./Icons";

// ---------------------------------------------------------------------------
// Per-treatment config helpers
// ---------------------------------------------------------------------------

/** Treatment IDs that carry a TOML config block the editor can surface. */
const CONFIGURABLE_IDS = new Set<string>([
  "url.qp.custom",
  "title.regex",
  "folder.regex",
]);

type ConfigValue = Record<string, unknown> | null;
type ConfigsMap = Record<string, ConfigValue>;

/** Default config seed when a configurable treatment is added with no value. */
function defaultConfigFor(id: string): ConfigValue {
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

/** Whether two ConfigsMap snapshots are equivalent (used for dirty checks). */
function configsEqual(a: ConfigsMap, b: ConfigsMap): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/**
 * Build the editor's config map from a freshly-loaded RuleSetDetail, seeding
 * a default block for any configurable treatment whose persisted config is
 * missing (e.g. a brand-new rule set or a duplicate of a built-in).
 */
function configsFromDetail(detail: RuleSetDetail): ConfigsMap {
  const out: ConfigsMap = {};
  for (const t of detail.treatments) {
    if (CONFIGURABLE_IDS.has(t.id)) {
      out[t.id] = (t.config as ConfigValue) ?? defaultConfigFor(t.id);
    }
  }
  return out;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function categoryLabel(cat: string): string {
  if (cat.startsWith("url")) return "URL";
  if (cat === "title") return "Title";
  if (cat === "folder_name") return "Folder";
  if (cat === "cross_field") return "Cross";
  return cat;
}

function categoryColor(cat: string): string {
  if (cat.startsWith("url"))   return "text-sky-400 bg-sky-400/10";
  if (cat === "title")         return "text-amber-400 bg-amber-400/10";
  if (cat === "folder_name")   return "text-emerald-400 bg-emerald-400/10";
  if (cat === "cross_field")   return "text-violet-400 bg-violet-400/10";
  return "text-neutral-400 bg-neutral-400/10";
}

// ---------------------------------------------------------------------------
// Sortable treatment row
// ---------------------------------------------------------------------------

interface TreatmentRowProps {
  id: string;
  info: TreatmentInfo | undefined;
  readonly?: boolean;
  config?: ConfigValue;
  onConfigChange?: (next: ConfigValue) => void;
  onRemove: () => void;
}

function TreatmentRow({
  id,
  info,
  readonly,
  config,
  onConfigChange,
  onRemove,
}: TreatmentRowProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    setActivatorNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id });

  const showConfig = CONFIGURABLE_IDS.has(id);

  return (
    <div
      ref={setNodeRef}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
        opacity: isDragging ? 0.4 : 1,
        zIndex: isDragging ? 10 : undefined,
      }}
      className="rounded hover:bg-surface-3 group select-none"
    >
      <div className="flex items-center gap-2 px-2 py-1.5">
        {/* Drag handle */}
        {!readonly ? (
          <button
            ref={setActivatorNodeRef}
            {...listeners}
            {...attributes}
            className="text-neutral-700 group-hover:text-neutral-500
                       hover:!text-neutral-300 cursor-grab active:cursor-grabbing
                       shrink-0 focus:outline-none"
            tabIndex={-1}
            aria-label="Drag to reorder"
          >
            <GripIcon className="w-3 h-3" />
          </button>
        ) : (
          <span className="w-3 shrink-0" />
        )}

        {/* Category badge */}
        {info && (
          <span
            className={`text-[9px] font-medium uppercase tracking-wider
                        px-1 rounded shrink-0 ${categoryColor(info.category)}`}
          >
            {categoryLabel(info.category)}
          </span>
        )}

        {/* Treatment name */}
        <span className="flex-1 min-w-0 text-xs text-neutral-200 truncate">
          {info?.name ?? id}
        </span>

        {/* Destructive badge */}
        {info?.destructive && (
          <span className="text-[9px] font-medium uppercase tracking-wider
                           px-1 rounded text-danger bg-danger/10 shrink-0">
            destructive
          </span>
        )}

        {/* Remove button */}
        {!readonly && (
          <button
            onClick={onRemove}
            className="text-neutral-700 hover:text-danger opacity-0
                       group-hover:opacity-100 transition-opacity shrink-0
                       focus:outline-none focus-visible:opacity-100"
            aria-label={`Remove ${info?.name ?? id}`}
          >
            <XIcon className="w-3 h-3" />
          </button>
        )}
      </div>

      {showConfig && (
        <TreatmentConfigEditor
          id={id}
          readonly={readonly}
          value={config ?? defaultConfigFor(id)}
          onChange={onConfigChange}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Per-treatment config editor (params list / regex pattern + replacement)
// ---------------------------------------------------------------------------

interface ConfigEditorProps {
  id: string;
  readonly?: boolean;
  value: ConfigValue;
  onChange?: (next: ConfigValue) => void;
}

function TreatmentConfigEditor({
  id,
  readonly,
  value,
  onChange,
}: ConfigEditorProps) {
  if (id === "url.qp.custom") {
    return (
      <ParamsListEditor
        readonly={readonly}
        value={value}
        onChange={onChange}
      />
    );
  }
  if (id === "title.regex" || id === "folder.regex") {
    return (
      <RegexConfigEditor
        readonly={readonly}
        value={value}
        onChange={onChange}
      />
    );
  }
  return null;
}

function ParamsListEditor({
  readonly,
  value,
  onChange,
}: {
  readonly?: boolean;
  value: ConfigValue;
  onChange?: (next: ConfigValue) => void;
}) {
  const params: string[] = useMemo(() => {
    const v = value && typeof value === "object" ? value.params : null;
    return Array.isArray(v) ? v.filter((s): s is string => typeof s === "string") : [];
  }, [value]);

  const [draft, setDraft] = useState("");

  const commit = (next: string[]) => {
    onChange?.({ params: next });
  };

  const addDraft = () => {
    const trimmed = draft.trim();
    if (!trimmed) return;
    if (params.includes(trimmed)) {
      setDraft("");
      return;
    }
    commit([...params, trimmed]);
    setDraft("");
  };

  if (readonly) {
    return (
      <div className="px-7 pb-2 -mt-0.5">
        {params.length === 0 ? (
          <p className="text-[10px] text-neutral-600 italic">
            No parameters configured.
          </p>
        ) : (
          <div className="flex flex-wrap gap-1">
            {params.map((p) => (
              <span
                key={p}
                className="text-[10px] font-mono text-neutral-300
                           bg-surface-3 border border-neutral-700
                           rounded px-1.5 py-0.5"
              >
                {p}
              </span>
            ))}
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="px-7 pb-2 -mt-0.5 space-y-1">
      <div className="flex flex-wrap gap-1">
        {params.map((p) => (
          <span
            key={p}
            className="text-[10px] font-mono text-neutral-300
                       bg-surface-3 border border-neutral-700
                       rounded pl-1.5 pr-0.5 py-0.5
                       flex items-center gap-1"
          >
            {p}
            <button
              type="button"
              onClick={() => commit(params.filter((q) => q !== p))}
              className="text-neutral-600 hover:text-danger
                         focus:outline-none rounded"
              aria-label={`Remove ${p}`}
            >
              <XIcon className="w-2.5 h-2.5" />
            </button>
          </span>
        ))}
      </div>
      <div className="flex items-center gap-1">
        <input
          type="text"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === ",") {
              e.preventDefault();
              addDraft();
            }
          }}
          placeholder="param_name…"
          className="flex-1 min-w-0 bg-surface-3 border border-neutral-700
                     rounded px-2 py-1 text-[11px] font-mono text-neutral-200
                     placeholder-neutral-600 focus:outline-none
                     focus:ring-1 focus:ring-accent"
        />
        <button
          type="button"
          onClick={addDraft}
          disabled={!draft.trim()}
          className="px-2 py-1 rounded text-[11px] text-neutral-400
                     hover:text-neutral-200 hover:bg-surface-3
                     disabled:opacity-40 disabled:hover:bg-transparent
                     focus:outline-none"
        >
          Add
        </button>
      </div>
    </div>
  );
}

function RegexConfigEditor({
  readonly,
  value,
  onChange,
}: {
  readonly?: boolean;
  value: ConfigValue;
  onChange?: (next: ConfigValue) => void;
}) {
  const obj = value && typeof value === "object" ? value : {};
  const pattern = typeof obj.pattern === "string" ? obj.pattern : "";
  const replacement = typeof obj.replacement === "string" ? obj.replacement : "";

  // Validate the regex client-side so the user sees mistakes inline rather
  // than at save-time.  Empty pattern is accepted (treated as no-op).
  const patternError = useMemo(() => {
    if (!pattern) return null;
    try {
      new RegExp(pattern);
      return null;
    } catch (e) {
      return (e as Error).message;
    }
  }, [pattern]);

  const update = (key: "pattern" | "replacement", v: string) => {
    onChange?.({ ...obj, [key]: v });
  };

  if (readonly) {
    return (
      <div className="px-7 pb-2 -mt-0.5 grid grid-cols-[auto_1fr] gap-x-2 gap-y-0.5">
        <span className="text-[10px] text-neutral-600 self-center">pattern</span>
        <span className="text-[10px] font-mono text-neutral-300 break-all">
          {pattern || <em className="text-neutral-600 not-italic">(empty)</em>}
        </span>
        <span className="text-[10px] text-neutral-600 self-center">replace</span>
        <span className="text-[10px] font-mono text-neutral-300 break-all">
          {replacement || <em className="text-neutral-600 not-italic">(empty)</em>}
        </span>
      </div>
    );
  }

  return (
    <div className="px-7 pb-2 -mt-0.5 space-y-1">
      <div className="flex items-center gap-2">
        <span className="text-[10px] text-neutral-500 w-14 shrink-0">pattern</span>
        <input
          type="text"
          value={pattern}
          onChange={(e) => update("pattern", e.target.value)}
          placeholder="^(.*) - Site name$"
          className={`flex-1 min-w-0 bg-surface-3 border rounded px-2 py-1
                      text-[11px] font-mono text-neutral-200
                      placeholder-neutral-600 focus:outline-none
                      focus:ring-1 ${patternError
                        ? "border-danger/60 focus:ring-danger"
                        : "border-neutral-700 focus:ring-accent"}`}
        />
      </div>
      <div className="flex items-center gap-2">
        <span className="text-[10px] text-neutral-500 w-14 shrink-0">replace</span>
        <input
          type="text"
          value={replacement}
          onChange={(e) => update("replacement", e.target.value)}
          placeholder="$1"
          className="flex-1 min-w-0 bg-surface-3 border border-neutral-700
                     rounded px-2 py-1 text-[11px] font-mono text-neutral-200
                     placeholder-neutral-600 focus:outline-none
                     focus:ring-1 focus:ring-accent"
        />
      </div>
      {patternError && (
        <p className="text-[10px] text-danger leading-snug">
          Invalid regex: {patternError}
        </p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Add-treatment picker
// ---------------------------------------------------------------------------

function AddTreatmentPicker({
  available,
  onAdd,
}: {
  available: TreatmentInfo[];
  onAdd: (id: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState("");
  const ref = useRef<HTMLDivElement>(null);

  // Close when clicking outside.
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
        setFilter("");
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const filtered = useMemo(() => {
    const q = filter.toLowerCase();
    return q
      ? available.filter(
          (t) =>
            t.name.toLowerCase().includes(q) ||
            t.id.toLowerCase().includes(q) ||
            categoryLabel(t.category).toLowerCase().includes(q),
        )
      : available;
  }, [available, filter]);

  // Group by category for display.
  const groups = useMemo(() => {
    const map = new Map<string, TreatmentInfo[]>();
    for (const t of filtered) {
      const key = categoryLabel(t.category);
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(t);
    }
    return map;
  }, [filtered]);

  if (available.length === 0) return null;

  return (
    <div ref={ref} className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        className="flex items-center gap-1.5 px-2 py-1 rounded
                   text-xs text-neutral-400 hover:text-neutral-200
                   hover:bg-surface-3 transition-colors focus:outline-none
                   focus-visible:ring-1 focus-visible:ring-accent"
      >
        <PlusIcon className="w-3 h-3" />
        Add treatment
      </button>

      {open && (
        <div
          className="absolute bottom-full left-0 mb-1 w-72 rounded
                     bg-surface-2 border border-neutral-700 shadow-xl
                     flex flex-col max-h-64 z-20"
        >
          {/* Filter input */}
          <div className="p-2 border-b border-neutral-700">
            <input
              autoFocus
              type="text"
              placeholder="Filter treatments…"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="w-full bg-surface-3 border border-neutral-700 rounded
                         px-2 py-1 text-xs text-neutral-200 placeholder-neutral-600
                         focus:outline-none focus:ring-1 focus:ring-accent"
            />
          </div>

          {/* Groups */}
          <div className="overflow-y-auto flex-1">
            {groups.size === 0 ? (
              <p className="px-3 py-2 text-xs text-neutral-600">
                No treatments match.
              </p>
            ) : (
              Array.from(groups.entries()).map(([group, items]) => (
                <div key={group}>
                  <p className="px-2 pt-2 pb-0.5 text-[10px] font-semibold
                                uppercase tracking-wider text-neutral-500">
                    {group}
                  </p>
                  {items.map((t) => (
                    <button
                      key={t.id}
                      onClick={() => {
                        onAdd(t.id);
                        setOpen(false);
                        setFilter("");
                      }}
                      className="w-full flex items-center gap-2 px-3 py-1.5
                                 text-xs text-neutral-300 hover:bg-surface-3
                                 hover:text-neutral-100 transition-colors
                                 text-left focus:outline-none
                                 focus-visible:bg-surface-3"
                    >
                      <span className="flex-1 truncate">{t.name}</span>
                      {t.destructive && (
                        <span className="text-[9px] text-danger shrink-0">
                          destructive
                        </span>
                      )}
                    </button>
                  ))}
                </div>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Right panel: treatment editor
// ---------------------------------------------------------------------------

interface TreatmentEditorProps {
  detail: RuleSetDetail;
  editedIds: string[];
  setEditedIds: React.Dispatch<React.SetStateAction<string[]>>;
  editedConfigs: ConfigsMap;
  setEditedConfigs: React.Dispatch<React.SetStateAction<ConfigsMap>>;
  dirty: boolean;
  saving: boolean;
  error: string | null;
  allTreatments: TreatmentInfo[];
  onSave: () => void;
  onDuplicate: () => void;
  onDiscard: () => void;
}

function TreatmentEditor({
  detail,
  editedIds,
  setEditedIds,
  editedConfigs,
  setEditedConfigs,
  dirty,
  saving,
  error,
  allTreatments,
  onSave,
  onDuplicate,
  onDiscard,
}: TreatmentEditorProps) {
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

  const treatmentMap = useMemo(
    () => new Map(allTreatments.map((t) => [t.id, t])),
    [allTreatments],
  );

  const available = useMemo(
    () => allTreatments.filter((t) => !editedIds.includes(t.id)),
    [allTreatments, editedIds],
  );

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    setEditedIds((prev) => {
      const oldIdx = prev.indexOf(String(active.id));
      const newIdx = prev.indexOf(String(over.id));
      return arrayMove(prev, oldIdx, newIdx);
    });
  }

  const isBuiltin = detail.is_builtin;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="px-4 py-3 border-b border-neutral-800 shrink-0
                      flex items-start justify-between gap-2">
        <div>
          <h2 className="text-sm font-semibold text-neutral-100">
            {detail.name}
          </h2>
          {isBuiltin ? (
            <div className="flex items-center gap-1 mt-0.5">
              <LockIcon className="w-2.5 h-2.5 text-neutral-500" />
              <span className="text-[10px] text-neutral-500">Built-in · read-only</span>
            </div>
          ) : (
            <p className="text-[10px] text-neutral-600 mt-0.5">
              {editedIds.length} treatment{editedIds.length !== 1 ? "s" : ""}
              {dirty && (
                <span className="ml-1.5 text-accent">· unsaved</span>
              )}
            </p>
          )}
        </div>

        <div className="flex items-center gap-1.5 shrink-0">
          {/* Duplicate */}
          <button
            onClick={onDuplicate}
            title="Duplicate rule set"
            className="flex items-center gap-1 px-2 py-1 rounded text-xs
                       text-neutral-400 hover:text-neutral-200
                       hover:bg-surface-3 transition-colors focus:outline-none
                       focus-visible:ring-1 focus-visible:ring-accent"
          >
            <CopyIcon className="w-3 h-3" />
            Duplicate
          </button>

          {/* Save / discard: user sets only */}
          {!isBuiltin && dirty && (
            <>
              <button
                onClick={onDiscard}
                className="px-2 py-1 rounded text-xs text-neutral-500
                           hover:text-neutral-300 transition-colors
                           focus:outline-none focus-visible:ring-1
                           focus-visible:ring-accent"
              >
                Discard
              </button>
              <button
                onClick={onSave}
                disabled={saving}
                className="px-3 py-1 rounded text-xs font-medium
                           bg-accent hover:bg-accent-hover text-neutral-950
                           disabled:opacity-50 transition-colors
                           focus:outline-none focus-visible:ring-2
                           focus-visible:ring-accent"
              >
                {saving ? "Saving…" : "Save"}
              </button>
            </>
          )}
        </div>
      </div>

      {/* Treatment list */}
      <div className="flex-1 overflow-y-auto px-2 py-2">
        {editedIds.length === 0 ? (
          <p className="px-2 py-4 text-xs text-neutral-600 text-center">
            {isBuiltin
              ? "No treatments in this rule set."
              : "No treatments yet. Add one below."}
          </p>
        ) : (
          <DndContext
            sensors={sensors}
            collisionDetection={closestCenter}
            onDragEnd={handleDragEnd}
          >
            <SortableContext
              items={editedIds}
              strategy={verticalListSortingStrategy}
            >
              {editedIds.map((id) => (
                <TreatmentRow
                  key={id}
                  id={id}
                  info={treatmentMap.get(id)}
                  readonly={isBuiltin}
                  config={editedConfigs[id]}
                  onConfigChange={(next) => {
                    setEditedConfigs((prev) => ({ ...prev, [id]: next }));
                  }}
                  onRemove={() => {
                    setEditedIds((prev) => prev.filter((x) => x !== id));
                    setEditedConfigs((prev) => {
                      if (!(id in prev)) return prev;
                      const next = { ...prev };
                      delete next[id];
                      return next;
                    });
                  }}
                />
              ))}
            </SortableContext>
          </DndContext>
        )}
      </div>

      {/* Footer: add treatment + error */}
      {!isBuiltin && (
        <div className="px-3 py-2 border-t border-neutral-800 shrink-0">
          {error && (
            <p className="mb-2 text-xs text-danger leading-snug">{error}</p>
          )}
          <AddTreatmentPicker
            available={available}
            onAdd={(id) => {
              setEditedIds((prev) => [...prev, id]);
              if (CONFIGURABLE_IDS.has(id)) {
                setEditedConfigs((prev) =>
                  id in prev ? prev : { ...prev, [id]: defaultConfigFor(id) },
                );
              }
            }}
          />
        </div>
      )}

      {/* Built-in duplicate CTA */}
      {isBuiltin && (
        <div className="px-4 py-3 border-t border-neutral-800 shrink-0">
          <button
            onClick={onDuplicate}
            className="w-full py-1.5 rounded text-xs font-medium
                       border border-neutral-700 text-neutral-300
                       hover:border-accent hover:text-accent transition-colors
                       focus:outline-none focus-visible:ring-2
                       focus-visible:ring-accent"
          >
            Duplicate to edit
          </button>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Left panel: rule-set list item
// ---------------------------------------------------------------------------

function RuleSetListItem({
  summary,
  selected,
  onClick,
}: {
  summary: RuleSetSummary;
  selected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={`w-full text-left px-3 py-2 rounded transition-colors
                  focus:outline-none focus-visible:ring-1 focus-visible:ring-accent
                  ${selected
                    ? "bg-surface-3 text-neutral-100"
                    : "text-neutral-400 hover:bg-surface-2 hover:text-neutral-200"
                  }`}
    >
      <div className="flex items-center gap-1.5 min-w-0">
        {summary.is_builtin && (
          <LockIcon className="w-2.5 h-2.5 text-neutral-600 shrink-0" />
        )}
        <span className="text-xs font-medium truncate">{summary.name}</span>
      </div>
      <p className="text-[10px] text-neutral-600 mt-0.5 truncate">
        {summary.treatment_count} treatment
        {summary.treatment_count !== 1 ? "s" : ""}
      </p>
    </button>
  );
}

// ---------------------------------------------------------------------------
// Inline name-input for new / duplicate flows
// ---------------------------------------------------------------------------

function InlineNameInput({
  placeholder,
  onConfirm,
  onCancel,
}: {
  placeholder: string;
  onConfirm: (name: string) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        const name = value.trim();
        if (name) onConfirm(name);
      }}
      className="flex items-center gap-1 px-2 py-1.5"
    >
      <input
        ref={inputRef}
        type="text"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder={placeholder}
        className="flex-1 min-w-0 bg-surface-3 border border-accent/40
                   rounded px-2 py-1 text-xs text-neutral-200
                   placeholder-neutral-600 focus:outline-none
                   focus:ring-1 focus:ring-accent"
      />
      <button
        type="submit"
        disabled={!value.trim()}
        className="px-2 py-1 rounded text-xs font-medium
                   bg-accent disabled:opacity-40 text-neutral-950
                   focus:outline-none"
      >
        OK
      </button>
      <button
        type="button"
        onClick={onCancel}
        className="px-1.5 py-1 text-neutral-500 hover:text-neutral-300
                   focus:outline-none text-xs"
      >
        ✕
      </button>
    </form>
  );
}

// ---------------------------------------------------------------------------
// Main modal
// ---------------------------------------------------------------------------

interface RuleSetEditorModalProps {
  open: boolean;
  initialSet?: string;
  onClose: () => void;
  /** Called after any mutation so parent can refresh its picker. */
  onRefreshList: () => void;
}

export function RuleSetEditorModal({
  open,
  initialSet,
  onClose,
  onRefreshList,
}: RuleSetEditorModalProps) {
  // ── Data ────────────────────────────────────────────────────────────────
  const [summaries, setSummaries] = useState<RuleSetSummary[]>([]);
  const [allTreatments, setAllTreatments] = useState<TreatmentInfo[]>([]);
  const [loadingData, setLoadingData] = useState(true);

  // ── Selection + edit state ───────────────────────────────────────────────
  const [selectedName, setSelectedName] = useState<string | null>(
    initialSet ?? null,
  );
  const [detail, setDetail] = useState<RuleSetDetail | null>(null);
  const [editedIds, setEditedIds] = useState<string[]>([]);
  const [savedIds, setSavedIds] = useState<string[]>([]);
  const [editedConfigs, setEditedConfigs] = useState<ConfigsMap>({});
  const [savedConfigs, setSavedConfigs] = useState<ConfigsMap>({});

  // ── Mutation state ───────────────────────────────────────────────────────
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // ── New / duplicate flows ────────────────────────────────────────────────
  const [newInput, setNewInput] = useState(false);
  const [dupSource, setDupSource] = useState<string | null>(null);

  // ── Deleting ─────────────────────────────────────────────────────────────
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  // ── Derived ─────────────────────────────────────────────────────────────
  const dirty = useMemo(() => {
    if (JSON.stringify(editedIds) !== JSON.stringify(savedIds)) return true;
    // Only treatments still in the rule set count toward config-dirty.
    const live: ConfigsMap = {};
    const liveSaved: ConfigsMap = {};
    for (const id of editedIds) {
      if (id in editedConfigs) live[id] = editedConfigs[id];
      if (id in savedConfigs) liveSaved[id] = savedConfigs[id];
    }
    return !configsEqual(live, liveSaved);
  }, [editedIds, savedIds, editedConfigs, savedConfigs]);

  // ── Load data on open ────────────────────────────────────────────────────
  const loadData = useCallback(async () => {
    setLoadingData(true);
    try {
      const [list, treats] = await Promise.all([
        ipc.listRuleSets(),
        ipc.listTreatments(),
      ]);
      setSummaries(list);
      setAllTreatments(treats);
    } catch {
      // Silently keep whatever we have.
    } finally {
      setLoadingData(false);
    }
  }, []);

  useEffect(() => {
    if (open) loadData();
  }, [open, loadData]);

  // Auto-select initialSet or first entry when summaries arrive.
  useEffect(() => {
    if (summaries.length === 0) return;
    if (!selectedName) {
      setSelectedName(summaries[0].name);
    }
  }, [summaries, selectedName]);

  // ── Load detail when selection changes ───────────────────────────────────
  useEffect(() => {
    if (!selectedName) {
      setDetail(null);
      setEditedIds([]);
      setSavedIds([]);
      setEditedConfigs({});
      setSavedConfigs({});
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const d = await ipc.getRuleSet(selectedName);
        if (cancelled) return;
        setDetail(d);
        setEditedIds(d.treatment_ids);
        setSavedIds(d.treatment_ids);
        const cfgs = configsFromDetail(d);
        setEditedConfigs(cfgs);
        setSavedConfigs(cfgs);
        setError(null);
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    })();
    return () => { cancelled = true; };
  }, [selectedName]);

  // ── Actions ──────────────────────────────────────────────────────────────

  const handleSave = async () => {
    if (!selectedName || !detail || detail.is_builtin) return;
    setSaving(true);
    setError(null);
    try {
      // Assemble RuleSetTreatment[] in the current edited order.  Non-config
      // treatments pass null; config treatments pass whatever the user entered
      // (falling back to a default when the entry was somehow dropped).
      const treatments: RuleSetTreatment[] = editedIds.map((id) => ({
        id,
        config: CONFIGURABLE_IDS.has(id)
          ? (editedConfigs[id] ?? defaultConfigFor(id))
          : null,
      }));

      const fresh = await ipc.saveRuleSetWithConfigs(selectedName, treatments);
      setDetail(fresh);
      setEditedIds(fresh.treatment_ids);
      setSavedIds(fresh.treatment_ids);
      const cfgs = configsFromDetail(fresh);
      setEditedConfigs(cfgs);
      setSavedConfigs(cfgs);

      // Update summary count in-place.
      setSummaries((prev) =>
        prev.map((s) =>
          s.name === selectedName
            ? { ...s, treatment_count: fresh.treatment_ids.length }
            : s,
        ),
      );
      onRefreshList();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleDiscard = () => {
    setEditedIds(savedIds);
    setEditedConfigs(savedConfigs);
    setError(null);
  };

  const handleDelete = async (name: string) => {
    try {
      await ipc.deleteRuleSet(name);
      setSummaries((prev) => prev.filter((s) => s.name !== name));
      if (selectedName === name) {
        setSelectedName(null);
        setDetail(null);
      }
      setConfirmDelete(null);
      onRefreshList();
    } catch (e) {
      setError(String(e));
      setConfirmDelete(null);
    }
  };

  const handleCreateConfirm = async (name: string) => {
    setNewInput(false);
    setError(null);
    try {
      await ipc.saveRuleSet(name, []);
      const fresh = await ipc.listRuleSets();
      setSummaries(fresh);
      setSelectedName(name);
      onRefreshList();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleDuplicateConfirm = async (newName: string) => {
    const source = dupSource ?? selectedName;
    setDupSource(null);
    if (!source) return;
    setError(null);
    try {
      await ipc.duplicateRuleSet(source, newName);
      const fresh = await ipc.listRuleSets();
      setSummaries(fresh);
      setSelectedName(newName);
      onRefreshList();
    } catch (e) {
      setError(String(e));
    }
  };

  // ── Close guard ──────────────────────────────────────────────────────────
  const handleClose = () => {
    if (dirty) {
      if (!window.confirm("You have unsaved changes. Discard and close?"))
        return;
    }
    onClose();
  };

  // Escape and Tab cycling: `handleClose` honours the dirty-confirm guard so
  // Escape behaves identically to clicking the overlay or the X button.
  const dialogRef = useFocusTrap<HTMLDivElement>(open, handleClose);

  if (!open) return null;

  // ── Render ───────────────────────────────────────────────────────────────
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center
                 bg-surface-0/80 backdrop-blur-sm animate-fade-in"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) handleClose();
      }}
    >
      <div
        ref={dialogRef}
        className="bg-surface-1 border border-neutral-800 rounded-lg
                   shadow-2xl flex overflow-hidden
                   w-[900px] max-w-[95vw] h-[620px] max-h-[90vh]"
        role="dialog"
        aria-modal="true"
        aria-label="Rule set editor"
      >
        {/* ── Left: rule-set list ─────────────────────────────────────── */}
        <div className="w-60 shrink-0 border-r border-neutral-800
                        flex flex-col bg-surface-0">
          {/* Header */}
          <div className="px-3 py-3 border-b border-neutral-800
                          flex items-center justify-between shrink-0">
            <h3 className="text-xs font-semibold text-neutral-400
                           uppercase tracking-wider">
              Rule Sets
            </h3>
            <button
              onClick={() => {
                setNewInput(true);
                setDupSource(null);
              }}
              title="New rule set"
              className="text-neutral-500 hover:text-neutral-200
                         transition-colors focus:outline-none
                         focus-visible:ring-1 focus-visible:ring-accent"
            >
              <PlusIcon className="w-3.5 h-3.5" />
            </button>
          </div>

          {/* New-set input */}
          {newInput && (
            <InlineNameInput
              placeholder="Rule set name…"
              onConfirm={handleCreateConfirm}
              onCancel={() => setNewInput(false)}
            />
          )}

          {/* Duplicate-set input */}
          {dupSource && (
            <InlineNameInput
              placeholder={`Copy of ${dupSource}…`}
              onConfirm={handleDuplicateConfirm}
              onCancel={() => setDupSource(null)}
            />
          )}

          {/* List */}
          <div className="flex-1 overflow-y-auto py-1 px-1 space-y-0.5">
            {loadingData && summaries.length === 0 ? (
              <p className="px-3 py-3 text-xs text-neutral-600">Loading…</p>
            ) : (
              summaries.map((s) => (
                <div key={s.name} className="group relative">
                  <RuleSetListItem
                    summary={s}
                    selected={selectedName === s.name}
                    onClick={() => {
                      if (selectedName === s.name) return;
                      if (dirty) {
                        if (!window.confirm("Discard unsaved changes?")) return;
                      }
                      setSelectedName(s.name);
                      setError(null);
                    }}
                  />

                  {/* Per-row actions (visible on hover) */}
                  <div className="absolute right-1 top-1/2 -translate-y-1/2
                                  hidden group-hover:flex items-center gap-0.5">
                    {/* Duplicate */}
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setDupSource(s.name);
                        setNewInput(false);
                      }}
                      title="Duplicate"
                      className="p-1 rounded text-neutral-600
                                 hover:text-neutral-300 hover:bg-surface-3
                                 focus:outline-none"
                    >
                      <CopyIcon className="w-2.5 h-2.5" />
                    </button>

                    {/* Delete: user sets only */}
                    {!s.is_builtin && (
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          setConfirmDelete(s.name);
                        }}
                        title="Delete"
                        className="p-1 rounded text-neutral-600
                                   hover:text-danger hover:bg-surface-3
                                   focus:outline-none"
                      >
                        <TrashIcon className="w-2.5 h-2.5" />
                      </button>
                    )}
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* ── Right: treatment editor ─────────────────────────────────── */}
        <div className="flex-1 flex flex-col min-w-0 relative">
          {/* Global close button (audit P2 #25): hover bg + 32x32 hit area */}
          <button
            onClick={handleClose}
            className="absolute top-3 right-3 z-10
                       inline-flex items-center justify-center w-8 h-8 rounded
                       text-neutral-400 hover:text-neutral-100
                       hover:bg-neutral-800/60 transition-colors
                       focus:outline-none focus-visible:ring-1
                       focus-visible:ring-accent"
            aria-label="Close editor"
          >
            <XIcon className="w-4 h-4" />
          </button>

          {detail ? (
            <TreatmentEditor
              detail={detail}
              editedIds={editedIds}
              setEditedIds={(updater) => {
                setEditedIds(updater);
              }}
              editedConfigs={editedConfigs}
              setEditedConfigs={setEditedConfigs}
              dirty={dirty}
              saving={saving}
              error={error}
              allTreatments={allTreatments}
              onSave={handleSave}
              onDuplicate={() => {
                setDupSource(selectedName);
                setNewInput(false);
              }}
              onDiscard={handleDiscard}
            />
          ) : (
            <div className="flex-1 flex items-center justify-center
                            text-xs text-neutral-600">
              {loadingData ? "Loading…" : "Select a rule set."}
            </div>
          )}
        </div>
      </div>

      {/* ── Delete-confirm overlay ──────────────────────────────────────── */}
      {confirmDelete && (
        <div className="fixed inset-0 z-60 flex items-center justify-center
                        bg-surface-0/70">
          <div className="bg-surface-2 border border-neutral-700 rounded-lg
                          p-5 w-72 shadow-xl">
            <p className="text-sm text-neutral-200 mb-1">Delete rule set?</p>
            <p className="text-xs text-neutral-500 mb-4">
              <span className="font-medium text-neutral-300">
                {confirmDelete}
              </span>{" "}
              will be permanently removed.
            </p>
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setConfirmDelete(null)}
                className="px-3 py-1.5 rounded text-xs text-neutral-400
                           hover:text-neutral-200 transition-colors
                           focus:outline-none"
              >
                Cancel
              </button>
              <button
                onClick={() => handleDelete(confirmDelete)}
                className="px-3 py-1.5 rounded text-xs font-medium
                           bg-danger/90 hover:bg-danger text-white
                           transition-colors focus:outline-none"
              >
                Delete
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
