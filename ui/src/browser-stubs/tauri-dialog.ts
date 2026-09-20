/**
 * Browser stub for @tauri-apps/plugin-dialog.
 *
 * The file-open dialog cannot be shown in a plain browser; returns null so
 * the library home "Open file…" button silently does nothing.
 */

export interface OpenDialogOptions {
  filters?: { name: string; extensions: string[] }[];
  multiple?: boolean;
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars
export function open(_options?: OpenDialogOptions): Promise<string | string[] | null> {
  console.warn("[browser-stub] open(): no Tauri runtime; file picker unavailable");
  return Promise.resolve(null);
}

export function save(): Promise<string | null> {
  console.warn("[browser-stub] save(): no Tauri runtime; file picker unavailable");
  return Promise.resolve(null);
}
