import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "path";

// https://vitejs.dev/config/
export default defineConfig(async () => {
  // TAURI_ENV_TARGET_TRIPLE is only set when the Tauri CLI drives Vite.
  // When absent we're running a plain browser preview; swap in no-op stubs
  // so the React tree mounts without a Tauri runtime.
  const isTauri = !!process.env.TAURI_ENV_TARGET_TRIPLE;
  const stubDir = resolve(__dirname, "src/browser-stubs");

  return {
    plugins: [react()],

    // Prevent Vite from obscuring Rust errors.
    clearScreen: false,

    resolve: isTauri
      ? {}
      : {
          alias: {
            "@tauri-apps/api/core": resolve(stubDir, "tauri-core.ts"),
            "@tauri-apps/api/window": resolve(stubDir, "tauri-window.ts"),
            "@tauri-apps/plugin-dialog": resolve(stubDir, "tauri-dialog.ts"),
            "@tauri-apps/plugin-shell": resolve(stubDir, "tauri-shell.ts"),
          },
        },

    server: {
      port: 5173,
      // Exit if port is already in use so the dev process doesn't start silently
      // on a different port and Tauri's WebView connects to the wrong endpoint.
      strictPort: true,
    },

    // TAURI_* env vars are injected by the Tauri CLI.
    envPrefix: ["VITE_", "TAURI_ENV_"],

    build: {
      // Tauri supports ES2021+ on all supported WebView2 versions.
      target: "chrome105",
      minify: process.env.TAURI_ENV_DEBUG ? false : "esbuild",
      sourcemap: !!process.env.TAURI_ENV_DEBUG,
    },
  };
});
