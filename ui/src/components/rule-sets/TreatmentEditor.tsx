import { useMemo, type Dispatch, type SetStateAction } from "react";
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
  verticalListSortingStrategy,
  arrayMove,
} from "@dnd-kit/sortable";
import type { RuleSetDetail, TreatmentInfo } from "../../ipc/types";
import { CopyIcon, LockIcon } from "../Icons";
import { primaryButton } from "../Modal";
import { AddTreatmentPicker } from "./AddTreatmentPicker";
import { TreatmentRow } from "./TreatmentRow";
import {
  CONFIGURABLE_IDS,
  defaultConfigFor,
  treatmentCount,
  type ConfigsMap,
} from "./treatments";

interface TreatmentEditorProps {
  detail: RuleSetDetail;
  ids: string[];
  setIds: Dispatch<SetStateAction<string[]>>;
  configs: ConfigsMap;
  setConfigs: Dispatch<SetStateAction<ConfigsMap>>;
  dirty: boolean;
  saving: boolean;
  error: string | null;
  allTreatments: TreatmentInfo[];
  onSave: () => void;
  onDuplicate: () => void;
  onDiscard: () => void;
}

/**
 * Right panel of the rule-set editor: the ordered, drag-sortable treatment
 * list of the selected set. Built-in sets render read-only and offer a
 * duplicate instead.
 */
export function TreatmentEditor({
  detail,
  ids,
  setIds,
  configs,
  setConfigs,
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
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  const treatmentMap = useMemo(
    () => new Map(allTreatments.map((t) => [t.id, t])),
    [allTreatments],
  );
  const available = useMemo(
    () => allTreatments.filter((t) => !ids.includes(t.id)),
    [allTreatments, ids],
  );

  const handleDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return;
    setIds((prev) => arrayMove(prev, prev.indexOf(String(active.id)), prev.indexOf(String(over.id))));
  };

  const add = (id: string) => {
    setIds((prev) => [...prev, id]);
    if (CONFIGURABLE_IDS.has(id)) {
      setConfigs((prev) => (id in prev ? prev : { ...prev, [id]: defaultConfigFor(id) }));
    }
  };

  const remove = (id: string) => {
    setIds((prev) => prev.filter((x) => x !== id));
    setConfigs((prev) => {
      if (!(id in prev)) return prev;
      const next = { ...prev };
      delete next[id];
      return next;
    });
  };

  const isBuiltin = detail.is_builtin;
  const errorLine = error && <p className="mb-2 text-xs text-danger leading-snug">{error}</p>;

  return (
    <div className="flex flex-col h-full">
      {/* Right padding keeps the actions clear of the modal's close button. */}
      <div className="pl-4 pr-14 py-3 border-b border-neutral-800 shrink-0 flex items-start justify-between gap-2">
        <div>
          <h2 className="text-sm font-semibold text-neutral-100">{detail.name}</h2>
          {isBuiltin ? (
            <div className="flex items-center gap-1 mt-0.5">
              <LockIcon className="w-2.5 h-2.5 text-neutral-500" />
              <span className="text-[10px] text-neutral-500">Built-in · read-only</span>
            </div>
          ) : (
            <p className="text-[10px] text-neutral-600 mt-0.5">
              {treatmentCount(ids.length)}
              {dirty && <span className="ml-1.5 text-accent">· unsaved</span>}
            </p>
          )}
        </div>

        <div className="flex items-center gap-1.5 shrink-0">
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
              <button onClick={onSave} disabled={saving} className={`px-3 py-1 ${primaryButton}`}>
                {saving ? "Saving…" : "Save"}
              </button>
            </>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-2 py-2">
        {ids.length === 0 ? (
          <p className="px-2 py-4 text-xs text-neutral-600 text-center">
            {isBuiltin ? "No treatments in this rule set." : "No treatments yet. Add one below."}
          </p>
        ) : (
          <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
            <SortableContext items={ids} strategy={verticalListSortingStrategy}>
              {ids.map((id) => (
                <TreatmentRow
                  key={id}
                  id={id}
                  info={treatmentMap.get(id)}
                  readonly={isBuiltin}
                  config={configs[id]}
                  onConfigChange={(next) => setConfigs((prev) => ({ ...prev, [id]: next }))}
                  onRemove={() => remove(id)}
                />
              ))}
            </SortableContext>
          </DndContext>
        )}
      </div>

      {isBuiltin ? (
        <div className="px-4 py-3 border-t border-neutral-800 shrink-0">
          {errorLine}
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
      ) : (
        <div className="px-3 py-2 border-t border-neutral-800 shrink-0">
          {errorLine}
          <AddTreatmentPicker available={available} onAdd={add} />
        </div>
      )}
    </div>
  );
}
