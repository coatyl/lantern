/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "Inter Variable",
          "Inter",
          "Segoe UI",
          "system-ui",
          "-apple-system",
          "sans-serif",
        ],
        display: [
          "Inter Variable",
          "Inter",
          "Segoe UI",
          "system-ui",
          "sans-serif",
        ],
        mono: ["JetBrains Mono", "Consolas", "monospace"],
      },
      colors: {
        // Surface colours: defined as CSS vars in index.css.
        // Hex fallbacks are baked into the CSS vars themselves.
        surface: {
          0: "var(--surface-0)",
          1: "var(--surface-1)",
          2: "var(--surface-2)",
          3: "var(--surface-3)",
          4: "var(--surface-4)",
        },
        // Theme-aware ink for branded surfaces (welcome, empty, wordmark).
        ink: {
          DEFAULT: "rgb(var(--ink) / <alpha-value>)",
          muted:   "rgb(var(--ink-muted) / <alpha-value>)",
          faint:   "rgb(var(--ink-faint) / <alpha-value>)",
        },
        // Accent and danger use the RGB-channel format so Tailwind opacity
        // modifiers (bg-accent/10, border-danger/30 etc.) keep working.
        accent: {
          DEFAULT: "rgb(var(--accent) / <alpha-value>)",
          hover:   "rgb(var(--accent-hover) / <alpha-value>)",
          glow:    "rgb(var(--accent-glow) / <alpha-value>)",
        },
        danger: "rgb(var(--danger) / <alpha-value>)",
        // Char-level diff highlighting in the sanitization preview.
        "diff-removed": "rgb(var(--diff-removed) / <alpha-value>)",
        "diff-added":   "rgb(var(--diff-added) / <alpha-value>)",
      },
      transitionDuration: {
        DEFAULT: "150ms",
      },
    },
  },
  plugins: [],
};
