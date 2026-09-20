/**
 * Split a recent-file path into the volume name (filename) and its
 * parent directory.  Handles both POSIX and Windows separators.
 *
 * Used by the library home collection; last-opened timestamps are not
 * on the `list_recent_files` IPC surface, so the UI only shows them
 * when a caller already has one.
 */

export interface VolumePresence {
  path: string;
  name: string;
  directory: string;
  /** ISO or display string; omitted when the backend only has a path. */
  lastOpened?: string;
}

export function parseVolumePath(path: string): VolumePresence {
  const normalized = path.replace(/\\/g, "/");
  const parts = normalized.split("/").filter((part) => part.length > 0);
  const name = parts.pop() ?? path;
  const directory = parts.join("/");
  return { path, name, directory };
}
