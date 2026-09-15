/**
 * Browser stub for @tauri-apps/plugin-shell.
 *
 * In browser preview mode, opening a URL falls back to window.open so you
 * can still click links while previewing the UI without a Tauri runtime.
 */

export async function open(url: string): Promise<void> {
  window.open(url, "_blank", "noopener,noreferrer");
}
