/**
 * Settings modal: a vertical tab rail with four panes.
 *   General:   theme, list density, dead-link opt-in, …
 *   Keyboard:  read-only shortcut reference
 *   Logs:      recent entries from the app log
 *   About:     version, build flavour, license
 *
 * Only the General pane edits settings, so Save (`update_settings`) is
 * offered on that pane alone.
 */

import { useState, useEffect, useRef, type KeyboardEvent } from "react";
import { ipc } from "../ipc";
import { useToast } from "../hooks/useToast";
import { useT } from "../i18n/I18nProvider";
import type { AppSettings } from "../ipc/types";
import { Modal, ModalCloseButton, primaryButton } from "./Modal";
import { GeneralPane } from "./settings-panes/GeneralPane";
import { KeyboardPane } from "./settings-panes/KeyboardPane";
import { LogsPane } from "./settings-panes/LogsPane";
import { AboutPane } from "./settings-panes/AboutPane";

type PaneId = "general" | "keyboard" | "logs" | "about";

const PANES: { id: PaneId; labelKey: string }[] = [
  { id: "general",  labelKey: "settings.pane.general"  },
  { id: "keyboard", labelKey: "settings.pane.keyboard" },
  { id: "logs",     labelKey: "settings.pane.logs"     },
  { id: "about",    labelKey: "settings.pane.about"    },
];

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

export function SettingsModal({ open, onClose }: SettingsModalProps) {
  const t = useT();
  const { toast } = useToast();
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  // Shown inline: without settings there is nothing else to render in the
  // General pane. Save errors go to a toast instead.
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [activePane, setActivePane] = useState<PaneId>("general");

  // Rail tabs, focused imperatively when arrow-keying.
  const tabRefs = useRef<Partial<Record<PaneId, HTMLButtonElement | null>>>({});

  useEffect(() => {
    if (!open) return;
    setLoading(true);
    setLoadError(null);
    ipc
      .getSettings()
      .then(setSettings)
      .catch((e) => setLoadError(String(e)))
      .finally(() => setLoading(false));
  }, [open]);

  const handleSave = async () => {
    if (!settings) return;
    setSaving(true);
    try {
      await ipc.updateSettings(settings);
      setSaved(true);
      setTimeout(() => setSaved(false), 1800);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setSaving(false);
    }
  };

  // Rail keyboard nav: ArrowUp/ArrowDown wrap, Home/End jump.
  const handleRailKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    const idx = PANES.findIndex((p) => p.id === activePane);
    let nextIdx: number;
    switch (event.key) {
      case "ArrowDown":
        nextIdx = (idx + 1) % PANES.length;
        break;
      case "ArrowUp":
        nextIdx = (idx - 1 + PANES.length) % PANES.length;
        break;
      case "Home":
        nextIdx = 0;
        break;
      case "End":
        nextIdx = PANES.length - 1;
        break;
      default:
        return;
    }
    event.preventDefault();
    const nextId = PANES[nextIdx].id;
    setActivePane(nextId);
    // Move focus too so the roving tabindex follows the selection.
    tabRefs.current[nextId]?.focus();
  };

  if (!open) return null;

  const showFooter = !!settings && !loading && activePane === "general";

  return (
    <Modal
      label={t("settings.title")}
      onClose={onClose}
      className="flex flex-col w-[720px] max-h-[90vh]"
    >
      <div className="flex items-center justify-between px-5 py-3.5 border-b border-neutral-800 shrink-0">
        <h2 className="text-sm font-semibold text-neutral-200">{t("settings.title")}</h2>
        <ModalCloseButton label={t("settings.close")} onClick={onClose} />
      </div>

      <div className="flex-1 flex min-h-0 overflow-hidden">
        <nav
          role="tablist"
          aria-label="Settings sections"
          aria-orientation="vertical"
          className="w-40 shrink-0 border-r border-neutral-800 bg-surface-2 py-2 overflow-y-auto"
        >
          {PANES.map(({ id, labelKey }) => {
            const isActive = activePane === id;
            return (
              <button
                key={id}
                ref={(el) => {
                  tabRefs.current[id] = el;
                }}
                role="tab"
                id={`settings-tab-${id}`}
                aria-controls={`settings-panel-${id}`}
                aria-selected={isActive}
                tabIndex={isActive ? 0 : -1}
                onClick={() => setActivePane(id)}
                onKeyDown={handleRailKeyDown}
                className={`w-full text-left px-4 py-1.5 text-xs
                            transition-colors focus:outline-none
                            focus-visible:ring-1 focus-visible:ring-accent border-l-2
                            ${isActive
                              ? "bg-accent/10 text-accent border-accent"
                              : "text-neutral-400 hover:text-neutral-200 hover:bg-surface-3 border-transparent"
                            }`}
              >
                {t(labelKey)}
              </button>
            );
          })}
        </nav>

        <section
          role="tabpanel"
          id={`settings-panel-${activePane}`}
          aria-labelledby={`settings-tab-${activePane}`}
          className="flex-1 overflow-y-auto px-5 py-4 min-h-0"
        >
          {activePane === "general" ? (
            loading ? (
              <p className="text-xs text-neutral-600">{t("loading.generic")}</p>
            ) : settings ? (
              <GeneralPane settings={settings} setSettings={setSettings} />
            ) : (
              <p className="text-xs text-danger">{loadError ?? "Could not load settings."}</p>
            )
          ) : activePane === "keyboard" ? (
            <KeyboardPane />
          ) : activePane === "logs" ? (
            <LogsPane />
          ) : (
            <AboutPane />
          )}
        </section>
      </div>

      {showFooter && (
        <div className="px-5 py-3 border-t border-neutral-800 shrink-0 flex items-center justify-between">
          <span
            className={`text-xs transition-opacity duration-300
                        ${saved ? "text-accent opacity-100" : "opacity-0"}`}
          >
            {t("common.saved")}
          </span>
          <div className="flex gap-2">
            <button
              onClick={onClose}
              className="px-3 py-1.5 rounded text-xs text-neutral-400
                         hover:text-neutral-200 transition-colors
                         focus:outline-none"
            >
              {t("common.cancel")}
            </button>
            <button onClick={handleSave} disabled={saving} className={`px-4 py-1.5 ${primaryButton}`}>
              {saving ? t("common.saving") : t("common.save")}
            </button>
          </div>
        </div>
      )}
    </Modal>
  );
}
