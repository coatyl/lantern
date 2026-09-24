/**
 * Browser stub for @tauri-apps/api/window.
 *
 * Returns a no-op window object so the TitleBar renders without crashing
 * when running outside of a Tauri WebView.
 */

const noop = () => Promise.resolve();

export function getCurrentWindow() {
  return {
    minimize: noop,
    toggleMaximize: noop,
    close: noop,
    maximize: noop,
    unmaximize: noop,
    isMaximized: () => Promise.resolve(false),
    isMinimized: () => Promise.resolve(false),
    isFocused: () => Promise.resolve(true),
    setFocus: noop,
    destroy: noop,
    onCloseRequested: () => Promise.resolve(() => {}),
  };
}
