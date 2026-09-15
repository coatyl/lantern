import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import { resolve } from "path";

const stubDir = resolve(__dirname, "src/browser-stubs");

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@tauri-apps/api/core": resolve(stubDir, "tauri-core.ts"),
      "@tauri-apps/api/window": resolve(stubDir, "tauri-window.ts"),
      "@tauri-apps/plugin-dialog": resolve(stubDir, "tauri-dialog.ts"),
      "@tauri-apps/plugin-shell": resolve(stubDir, "tauri-shell.ts"),
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
    css: false,
  },
});
