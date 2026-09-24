/**
 * General settings pane: Theme, List density, Recent files, Session
 * recovery, Dead-link checker opt-in, Settings location.
 *
 * Lifted unchanged from the previous flat `SettingsModal.tsx`.  Operates
 * directly on the shared `settings` state owned by the parent modal so
 * the Save button there can persist any edit made here.
 */

import { open as shellOpen } from "@tauri-apps/plugin-shell";
import type { AppSettings, ThemeSetting } from "../../ipc/types";

const THEMES: { value: ThemeSetting; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light",  label: "Light"  },
  { value: "dark",   label: "Dark"   },
];

function Label({ children }: { children: React.ReactNode }) {
  return (
    <p className="text-[10px] font-semibold uppercase tracking-wider
                  text-neutral-500 mb-1.5">
      {children}
    </p>
  );
}

function Divider() {
  return <hr className="border-neutral-800 my-4" />;
}

interface GeneralPaneProps {
  settings: AppSettings;
  setSettings: React.Dispatch<React.SetStateAction<AppSettings | null>>;
}

export function GeneralPane({ settings, setSettings }: GeneralPaneProps) {
  const handleOpenFolder = async () => {
    if (!settings.settings_path) return;
    const folder = settings.settings_path
      .replace(/\\/g, "/")
      .split("/")
      .slice(0, -1)
      .join("/");
    try {
      await shellOpen(folder);
    } catch {
      try { await shellOpen(settings.settings_path); } catch { /* ignore */ }
    }
  };

  return (
    <section aria-label="General settings">
      {/* ── Theme ─────────────────────────────────────────────────────────── */}
      <section>
        <Label>Appearance</Label>
        <div className="flex gap-2">
          {THEMES.map(({ value, label }) => (
            <button
              key={value}
              onClick={() =>
                setSettings((s) => (s ? { ...s, theme: value } : s))
              }
              className={`flex-1 py-1.5 rounded border text-xs
                          transition-colors focus:outline-none
                          focus-visible:ring-1 focus-visible:ring-accent
                          ${settings.theme === value
                            ? "border-accent/50 bg-accent/10 text-accent"
                            : "border-neutral-700 text-neutral-500 hover:border-neutral-500 hover:text-neutral-300"
                          }`}
            >
              {label}
            </button>
          ))}
        </div>
        <p className="mt-1.5 text-[10px] text-neutral-700 leading-snug">
          Theme preference is stored here for the desktop app shell.
        </p>
      </section>

      <Divider />

      {/* ── List density ──────────────────────────────────────────────────── */}
      <section>
        <Label>List density</Label>
        <div className="flex gap-2">
          {(["compact", "comfortable"] as const).map((value) => (
            <button
              key={value}
              onClick={() =>
                setSettings((s) => (s ? { ...s, list_density: value } : s))
              }
              className={`flex-1 py-1.5 rounded border text-xs capitalize
                          transition-colors focus:outline-none
                          focus-visible:ring-1 focus-visible:ring-accent
                          ${settings.list_density === value
                            ? "border-accent/50 bg-accent/10 text-accent"
                            : "border-neutral-700 text-neutral-500 hover:border-neutral-500 hover:text-neutral-300"
                          }`}
            >
              {value}
            </button>
          ))}
        </div>
        <p className="mt-1.5 text-[10px] text-neutral-700 leading-snug">
          Comfortable mode adds breathing room around list rows
          (~36 px tall instead of ~28 px).
        </p>
      </section>

      <Divider />

      {/* ── Recent files ──────────────────────────────────────────────────── */}
      <section>
        <Label>Recent files</Label>
        <div className="flex items-center gap-3">
          <input
            type="range"
            min={0}
            max={50}
            step={1}
            value={settings.recent_files_max}
            onChange={(e) =>
              setSettings((s) =>
                s ? { ...s, recent_files_max: Number(e.target.value) } : s,
              )
            }
            className="flex-1"
          />
          <span className="w-10 text-center text-xs text-neutral-300 shrink-0">
            {settings.recent_files_max === 0
              ? "Off"
              : settings.recent_files_max}
          </span>
        </div>
        <p className="mt-1 text-[10px] text-neutral-700">
          {settings.recent_files_max === 0
            ? "Recent files list disabled."
            : `Show up to ${settings.recent_files_max} recent files on the library home.`}
        </p>
      </section>

      <Divider />

      {/* ── Crash recovery ────────────────────────────────────────────────── */}
      <section>
        <Label>Session recovery</Label>
        <label className="flex items-start gap-3 cursor-pointer group">
          <div className="relative shrink-0 mt-0.5">
            <input
              type="checkbox"
              checked={settings.crash_recovery_enabled}
              onChange={(e) =>
                setSettings((s) =>
                  s ? { ...s, crash_recovery_enabled: e.target.checked } : s,
                )
              }
              className="sr-only peer"
            />
            <div className="w-8 h-4 rounded-full border border-neutral-700
                            bg-surface-3 peer-checked:bg-accent/30
                            peer-checked:border-accent/50
                            transition-colors" />
            <div className="absolute top-0.5 left-0.5 w-3 h-3 rounded-full
                            bg-neutral-600 peer-checked:bg-accent
                            peer-checked:translate-x-4 transition-all" />
          </div>
          <div>
            <p className="text-xs text-neutral-300 leading-snug
                          group-hover:text-neutral-100 transition-colors">
              Reopen files from previous session
            </p>
            <p className="text-[10px] text-neutral-600 mt-0.5">
              When Lantern closes unexpectedly, offer to restore open files
              on next launch.
            </p>
          </div>
        </label>
      </section>

      <Divider />

      {/* ── Dead-link checker ────────────────────────────────────────────── */}
      <section>
        <Label>Dead-link checker</Label>
        <label className="flex items-start gap-3 cursor-pointer group">
          <div className="relative shrink-0 mt-0.5">
            <input
              type="checkbox"
              checked={settings.dead_link_checker_opt_in}
              onChange={(e) =>
                setSettings((s) =>
                  s ? { ...s, dead_link_checker_opt_in: e.target.checked } : s,
                )
              }
              className="sr-only peer"
            />
            <div className="w-8 h-4 rounded-full border border-neutral-700
                            bg-surface-3 peer-checked:bg-accent/30
                            peer-checked:border-accent/50
                            transition-colors" />
            <div className="absolute top-0.5 left-0.5 w-3 h-3 rounded-full
                            bg-neutral-600 peer-checked:bg-accent
                            peer-checked:translate-x-4 transition-all" />
          </div>
          <div>
            <p className="text-xs text-neutral-300 leading-snug
                          group-hover:text-neutral-100 transition-colors">
              Check bookmarks for dead links
            </p>
            <p className="text-[10px] text-neutral-600 mt-0.5">
              When enabled, the Tools menu can probe every bookmark for
              reachability. All network requests stay opt-in
              (PRD NFR-PRIV-2).
            </p>
          </div>
        </label>
      </section>

      <Divider />

      {/* ── Settings location ─────────────────────────────────────────────── */}
      <section>
        <Label>Settings location</Label>
        <div className="flex items-center gap-2 p-2 rounded bg-surface-2
                        border border-neutral-800">
          <code className="flex-1 min-w-0 text-[10px] text-neutral-500
                           font-mono truncate" title={settings.settings_path}>
            {settings.settings_path}
          </code>
          <button
            onClick={handleOpenFolder}
            className="shrink-0 text-[10px] text-neutral-500
                       hover:text-neutral-200 transition-colors
                       focus:outline-none focus-visible:ring-1
                       focus-visible:ring-accent rounded px-1.5 py-0.5
                       border border-neutral-700 hover:border-neutral-500"
          >
            Open folder
          </button>
        </div>

        <div className="mt-2 flex items-center gap-2 p-2 rounded bg-surface-2
                        border border-neutral-800">
          <p className="text-[10px] text-neutral-600 shrink-0">Rules dir:</p>
          <code className="flex-1 min-w-0 text-[10px] text-neutral-500
                           font-mono truncate" title={settings.rules_dir}>
            {settings.rules_dir}
          </code>
        </div>
      </section>
    </section>
  );
}
