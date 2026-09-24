/**
 * Command palette: a keyboard-first "do anything" surface.
 *
 * Opened with Ctrl+K (⌘K).  Focus is trapped inside the dialog; the filter
 * input keeps focus while ArrowUp/ArrowDown move the highlighted row,
 * Enter runs the highlighted command, and Escape (via useFocusTrap)
 * closes and restores the previously-focused element.
 */

import { useEffect, useMemo, useState, type FormEvent, type KeyboardEvent } from "react";

import { useFocusTrap } from "../hooks/useFocusTrap";
import { useT } from "../i18n/I18nProvider";
import {
  filterCommands,
  stepActiveIndex,
  type CommandId,
  type PaletteCommand,
} from "./commandPalette";

export interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  commands: PaletteCommand[];
  onRun: (id: CommandId) => void;
}

export function CommandPalette({
  open,
  onClose,
  commands,
  onRun,
}: CommandPaletteProps) {
  const t = useT();
  const dialogRef = useFocusTrap<HTMLDivElement>(open, onClose);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setActiveIndex(0);
  }, [open]);

  const filtered = useMemo(
    () => filterCommands(commands, query),
    [commands, query],
  );
  const clampedIndex =
    filtered.length === 0 ? 0 : Math.min(activeIndex, filtered.length - 1);

  useEffect(() => {
    if (!open) return;
    const row = filtered[clampedIndex];
    if (!row) return;
    const el = document.getElementById(optionId(row.id));
    el?.scrollIntoView?.({ block: "nearest" });
  }, [open, clampedIndex, filtered]);

  if (!open) return null;

  const active = filtered[clampedIndex] ?? null;

  const runActive = () => {
    if (!active || !active.enabled) return;
    onRun(active.id);
  };

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    runActive();
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveIndex(stepActiveIndex(clampedIndex, 1, filtered.length));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex(stepActiveIndex(clampedIndex, -1, filtered.length));
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      setActiveIndex(0);
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      if (filtered.length > 0) setActiveIndex(filtered.length - 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      runActive();
    }
  };

  return (
    <div
      className="fixed inset-0 z-[60] flex items-start justify-center
                 bg-scrim/60 backdrop-blur-sm animate-fade-in pt-[12vh]"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        className="bg-surface-1 border border-neutral-800 rounded-lg
                   shadow-2xl w-[520px] max-w-[95vw] max-h-[70vh]
                   flex flex-col overflow-hidden"
        role="dialog"
        aria-modal="true"
        aria-label={t("commandPalette.title")}
      >
        <form onSubmit={handleSubmit} className="shrink-0">
          <input
            type="search"
            role="combobox"
            aria-expanded="true"
            aria-controls="command-palette-list"
            aria-activedescendant={active ? optionId(active.id) : undefined}
            aria-autocomplete="list"
            placeholder={t("commandPalette.placeholder")}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setActiveIndex(0);
            }}
            onKeyDown={handleKeyDown}
            className="w-full bg-transparent text-sm text-neutral-100
                       px-4 py-3 placeholder-neutral-600
                       focus:outline-none border-b border-neutral-800"
          />
        </form>

        <ul
          id="command-palette-list"
          role="listbox"
          aria-label={t("commandPalette.title")}
          className="flex-1 overflow-y-auto py-1 min-h-0"
        >
          {filtered.length === 0 ? (
            <li className="px-4 py-3 text-xs text-neutral-500">
              {t("commandPalette.empty")}
            </li>
          ) : (
            filtered.map((cmd, idx) => {
              const isActive = idx === clampedIndex;
              return (
                <li
                  key={cmd.id}
                  id={optionId(cmd.id)}
                  role="option"
                  aria-selected={isActive}
                  aria-disabled={cmd.enabled ? undefined : true}
                  onMouseEnter={() => setActiveIndex(idx)}
                  onMouseDown={(e) => {
                    // Keep the input focused; click still runs the command.
                    e.preventDefault();
                    if (cmd.enabled) onRun(cmd.id);
                  }}
                  className={`flex items-center justify-between gap-3
                              px-4 py-2 text-sm cursor-default select-none
                              ${isActive ? "bg-accent/10 text-accent" : "text-neutral-200"}
                              ${cmd.enabled ? "" : "opacity-40"}`}
                >
                  <span>{cmd.label}</span>
                  {cmd.shortcut && (
                    <kbd
                      className={`font-mono text-[11px] shrink-0
                                  ${isActive ? "text-accent/80" : "text-neutral-500"}`}
                    >
                      {cmd.shortcut}
                    </kbd>
                  )}
                </li>
              );
            })
          )}
        </ul>

        <div className="px-4 py-2 border-t border-neutral-800 shrink-0
                        flex items-center justify-between text-[10px] text-neutral-600">
          <span>{t("commandPalette.hint")}</span>
          <kbd className="font-mono text-neutral-500">
            {t("commandPalette.shortcut")}
          </kbd>
        </div>
      </div>
    </div>
  );
}

function optionId(id: CommandId): string {
  return `command-palette-${id}`;
}

export { filterCommands, stepActiveIndex } from "./commandPalette";
export type { CommandId, PaletteCommand } from "./commandPalette";
