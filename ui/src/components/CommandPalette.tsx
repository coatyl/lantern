/**
 * Command palette: a keyboard-first "do anything" surface, opened with
 * Ctrl+K (⌘K). The filter input keeps focus while ArrowUp/ArrowDown move the
 * highlighted row, Enter runs it, and Escape closes the palette and restores
 * the previously-focused element.
 */

import { useEffect, useMemo, useState, type KeyboardEvent } from "react";

import { useT } from "../i18n/I18nProvider";
import { Modal } from "./Modal";
import {
  filterCommands,
  stepActiveIndex,
  type CommandId,
  type PaletteCommand,
} from "./paletteCommands";

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  commands: PaletteCommand[];
  onRun: (id: CommandId) => void;
}

export function CommandPalette({ open, ...props }: CommandPaletteProps) {
  // Mounting on open gives every session a fresh query and highlight.
  return open ? <PaletteDialog {...props} /> : null;
}

function PaletteDialog({ onClose, commands, onRun }: Omit<CommandPaletteProps, "open">) {
  const t = useT();
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);

  const filtered = useMemo(() => filterCommands(commands, query), [commands, query]);
  const clampedIndex = filtered.length === 0 ? 0 : Math.min(activeIndex, filtered.length - 1);
  const active = filtered[clampedIndex] ?? null;

  useEffect(() => {
    if (!active) return;
    document.getElementById(optionId(active.id))?.scrollIntoView?.({ block: "nearest" });
  }, [active]);

  const run = (cmd: PaletteCommand | null) => {
    if (cmd?.enabled) onRun(cmd.id);
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    let next: number;
    switch (event.key) {
      case "ArrowDown":
        next = stepActiveIndex(clampedIndex, 1, filtered.length);
        break;
      case "ArrowUp":
        next = stepActiveIndex(clampedIndex, -1, filtered.length);
        break;
      case "Home":
        next = 0;
        break;
      case "End":
        next = Math.max(filtered.length - 1, 0);
        break;
      case "Enter":
        event.preventDefault();
        run(active);
        return;
      default:
        return;
    }
    event.preventDefault();
    setActiveIndex(next);
  };

  return (
    <Modal
      label={t("commandPalette.title")}
      onClose={onClose}
      placement="z-[60] items-start pt-[12vh]"
      className="flex flex-col w-[520px] max-h-[70vh]"
    >
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
        className="shrink-0 w-full bg-transparent text-sm text-neutral-100
                   px-4 py-3 placeholder-neutral-600
                   focus:outline-none border-b border-neutral-800"
      />

      <ul
        id="command-palette-list"
        role="listbox"
        aria-label={t("commandPalette.title")}
        className="flex-1 overflow-y-auto py-1 min-h-0"
      >
        {filtered.length === 0 ? (
          <li className="px-4 py-3 text-xs text-neutral-500">{t("commandPalette.empty")}</li>
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
                  // Keep the input focused; the click still runs the command.
                  e.preventDefault();
                  run(cmd);
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
        <kbd className="font-mono text-neutral-500">{t("commandPalette.shortcut")}</kbd>
      </div>
    </Modal>
  );
}

function optionId(id: CommandId): string {
  return `command-palette-${id}`;
}
