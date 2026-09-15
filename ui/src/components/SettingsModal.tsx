/**
 * Settings modal, left-rail layout with four panes:
 *   General:   theme / list density / dead-link opt-in / etc.
 *   Keyboard:  read-only shortcut reference
 *   Logs:      last entries from `<settings_dir>/logs/lantern.log`
 *   About:     build flavor / version / license
 *
 * Settings are persisted to `%APPDATA%\Lantern\settings.toml` via the
 * `get_settings` / `update_settings` Tauri commands; only the General pane
 * mutates settings so Save is wired solely to that path.
 */

import { useState, useEffect, useCallback, useRef, type KeyboardEvent } from "react";
import { ipc } from "../ipc";
import { useFocusTrap } from "../hooks/useFocusTrap";
import { useToast } from "../hooks/useToast";
import { useT } from "../i18n/I18nProvider";
import type { AppSettings } from "../ipc/types";
import { XIcon } from "./Icons";
import { GeneralPane } from "./settings-panes/GeneralPane";
import { KeyboardPane } from "./settings-panes/KeyboardPane";
import { LogsPane } from "./settings-panes/LogsPane";
import { AboutPane } from "./settings-panes/AboutPane";

type PaneId = "general" | "keyboard" | "logs" | "about";

/**
 * Pane order is stable; the user-facing label is resolved through `t(...)`
 * inside the render so the rail responds to locale changes.  The i18n key
 * lives next to the id rather than the literal label.
 */
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
  // The General-pane load error is the only inline error we still surface;
  // when settings can't be read we can't render the pane, so showing the
  // failure inline lets the user act on it without competing with a toast.
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [activePane, setActivePane] = useState<PaneId>("general");

  // Focus trap + Escape handler: A11Y_AUDIT_v0.0.6 P0 #4.  The hook restores
  // focus to whatever opened the modal (typically the title-bar settings
  // button) once the dialog unmounts.
  const dialogRef = useFocusTrap<HTMLDivElement>(open, onClose);

  // Refs for each rail tab, used to imperatively focus when arrow-keying.
  const tabRefs = useRef<Record<PaneId, HTMLButtonElement | null>>({
    general: null,
    keyboard: null,
    logs: null,
    about: null,
  });

  // ── Load ────────────────────────────────────────────────────────────────
  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const s = await ipc.getSettings();
      setSettings(s);
    } catch (e) {
      setLoadError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (open) load();
  }, [open, load]);

  // ── Save ────────────────────────────────────────────────────────────────
  const handleSave = async () => {
    if (!settings) return;
    setSaving(true);
    try {
      await ipc.updateSettings(settings);
      setSaved(true);
      setTimeout(() => setSaved(false), 1800);
    } catch (e) {
      // Save failures route through the global toast surface so the modal's
      // primary affordance (the form itself) stays uncluttered.  v0.0.11 QoL
      // slice 1.
      toast(String(e), "error");
    } finally {
      setSaving(false);
    }
  };

  // ── Rail keyboard nav (ArrowUp/ArrowDown wrap, Home/End jump) ───────────
  const handleRailKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    const idx = PANES.findIndex((p) => p.id === activePane);
    if (idx < 0) return;

    let nextIdx: number | null = null;
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
    // Move focus too so the roving tabindex stays in sync visually.
    tabRefs.current[nextId]?.focus();
  };

  if (!open) return null;

  const showFooter = !!settings && !loading && activePane === "general";

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center
                 bg-black/60 backdrop-blur-sm animate-fade-in"
      onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div
        ref={dialogRef}
        className="bg-surface-1 border border-neutral-800 rounded-lg
                   shadow-2xl w-[720px] max-w-[95vw] max-h-[90vh]
                   flex flex-col overflow-hidden"
        role="dialog"
        aria-modal="true"
        aria-label={t("settings.title")}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3.5
                        border-b border-neutral-800 shrink-0">
          <h2 className="text-sm font-semibold text-neutral-200">{t("settings.title")}</h2>
          {/* audit P2 #25: hover bg + 32x32 hit area on modal close button */}
          <button
            onClick={onClose}
            className="inline-flex items-center justify-center w-8 h-8 rounded
                       text-neutral-400 hover:text-neutral-100
                       hover:bg-neutral-800/60 transition-colors
                       focus:outline-none focus-visible:ring-1 focus-visible:ring-accent"
            aria-label={t("settings.close")}
          >
            <XIcon className="w-4 h-4" />
          </button>
        </div>

        {/* Body: left rail + right pane */}
        <div className="flex-1 flex min-h-0 overflow-hidden">
          {/* Left rail */}
          <nav
            role="tablist"
            aria-label="Settings sections"
            aria-orientation="vertical"
            className="w-40 shrink-0 border-r border-neutral-800
                       bg-surface-2 py-2 overflow-y-auto"
          >
            {PANES.map(({ id, labelKey }) => {
              const isActive = activePane === id;
              return (
                <button
                  key={id}
                  ref={(el) => { tabRefs.current[id] = el; }}
                  role="tab"
                  id={`settings-tab-${id}`}
                  aria-controls={`settings-panel-${id}`}
                  aria-selected={isActive ? "true" : "false"}
                  tabIndex={isActive ? 0 : -1}
                  onClick={() => setActivePane(id)}
                  onKeyDown={handleRailKeyDown}
                  className={`w-full text-left px-4 py-1.5 text-xs
                              transition-colors focus:outline-none
                              focus-visible:ring-1 focus-visible:ring-accent
                              ${isActive
                                ? "bg-accent/10 text-accent border-l-2 border-accent"
                                : "text-neutral-400 hover:text-neutral-200 hover:bg-surface-3 border-l-2 border-transparent"
                              }`}
                >
                  {t(labelKey)}
                </button>
              );
            })}
          </nav>

          {/* Right pane */}
          <section
            role="tabpanel"
            id={`settings-panel-${activePane}`}
            aria-labelledby={`settings-tab-${activePane}`}
            className="flex-1 overflow-y-auto px-5 py-4 min-h-0"
          >
            {activePane === "general" ? (
              loading ? (
                <p className="text-xs text-neutral-600">{t("loading.generic")}</p>
              ) : !settings ? (
                // Hard load failure, kept inline because without a settings
                // object there's literally nothing else to render in the
                // pane.  Save errors still go through the toast surface.
                <p className="text-xs text-danger">
                  {loadError ?? "Could not load settings."}
                </p>
              ) : (
                <GeneralPane settings={settings} setSettings={setSettings} />
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

        {/* Footer: only meaningful for General (the only mutable pane) */}
        {showFooter && (
          <div className="px-5 py-3 border-t border-neutral-800 shrink-0
                          flex items-center justify-between">
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
              <button
                onClick={handleSave}
                disabled={saving}
                className="px-4 py-1.5 rounded text-xs font-medium
                           bg-accent hover:bg-accent-hover text-neutral-950
                           disabled:opacity-50 transition-colors
                           focus:outline-none focus-visible:ring-2
                           focus-visible:ring-accent"
              >
                {saving ? t("common.saving") : t("common.save")}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
