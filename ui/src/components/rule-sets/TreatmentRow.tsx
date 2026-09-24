import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { TreatmentInfo } from "../../ipc/types";
import { GripIcon, XIcon } from "../Icons";
import { TreatmentConfigEditor } from "./TreatmentConfigEditor";
import {
  CONFIGURABLE_IDS,
  categoryColor,
  categoryLabel,
  defaultConfigFor,
  type ConfigValue,
} from "./treatments";

interface TreatmentRowProps {
  id: string;
  info: TreatmentInfo | undefined;
  readonly: boolean;
  config: ConfigValue | undefined;
  onConfigChange: (next: ConfigValue) => void;
  onRemove: () => void;
}

/** One drag-sortable treatment in the editor, with its inline config. */
export function TreatmentRow({
  id,
  info,
  readonly,
  config,
  onConfigChange,
  onRemove,
}: TreatmentRowProps) {
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } =
    useSortable({ id });

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
        {readonly ? (
          <span className="w-3 shrink-0" />
        ) : (
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
        )}

        {info && (
          <span
            className={`text-[9px] font-medium uppercase tracking-wider
                        px-1 rounded shrink-0 ${categoryColor(info.category)}`}
          >
            {categoryLabel(info.category)}
          </span>
        )}

        <span className="flex-1 min-w-0 text-xs text-neutral-200 truncate">{info?.name ?? id}</span>

        {info?.destructive && (
          <span className="text-[9px] font-medium uppercase tracking-wider
                           px-1 rounded text-danger bg-danger/10 shrink-0">
            destructive
          </span>
        )}

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

      {CONFIGURABLE_IDS.has(id) && (
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
