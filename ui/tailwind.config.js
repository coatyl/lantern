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
      // Bare `border` / `divide` utilities pick up the theme's divider
      // colour instead of Tailwind's default gray-200, which drew bright
      // white rules under the title, tab and status bars in dark mode.
      borderColor: {
        DEFAULT: "rgb(var(--line) / <alpha-value>)",
      },
      divideColor: {
        DEFAULT: "rgb(var(--line) / <alpha-value>)",
      },
      colors: {
        // The whole UI was written against Tailwind's neutral scale for the
        // dark theme.  Rather than rewrite ~500 class names, the scale itself
        // is theme-aware: each step is a CSS variable that index.css defines
        // once per theme (dark ramp: 50 = lightest; light ramp mirrored, so
        // `text-neutral-200` is primary ink in both rooms).  Steps carry
        // contrast floors against surface-0, documented next to the values.
        neutral: {
          50:  "rgb(var(--n-50) / <alpha-value>)",
          100: "rgb(var(--n-100) / <alpha-value>)",
          200: "rgb(var(--n-200) / <alpha-value>)",
          300: "rgb(var(--n-300) / <alpha-value>)",
          400: "rgb(var(--n-400) / <alpha-value>)",
          500: "rgb(var(--n-500) / <alpha-value>)",
          600: "rgb(var(--n-600) / <alpha-value>)",
          700: "rgb(var(--n-700) / <alpha-value>)",
          800: "rgb(var(--n-800) / <alpha-value>)",
          900: "rgb(var(--n-900) / <alpha-value>)",
          950: "rgb(var(--n-950) / <alpha-value>)",
        },
        // Text on filled accent / danger buttons, and the modal backdrop.
        "on-accent": "rgb(var(--on-accent) / <alpha-value>)",
        "on-danger": "rgb(var(--on-danger) / <alpha-value>)",
        scrim: "rgb(var(--scrim) / <alpha-value>)",
        // Category tones (rule-set treatment chips, log levels, badges).
        info: "rgb(var(--tone-info) / <alpha-value>)",
        warn: "rgb(var(--tone-warn) / <alpha-value>)",
        ok:   "rgb(var(--tone-ok) / <alpha-value>)",
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
