/**
 * I18n context + provider: closes NFR-L-1.
 *
 * Wrap the React tree once (in `main.tsx`) and use the `useT()` hook to
 * resolve dot-separated keys to the active locale's string.  The provider
 * is intentionally tiny (no async loading, no fallback chain, no
 * pluralisation) because v0.0.9 only ships English.  Adding a second
 * locale is a drop-in file in `./locales.ts`.
 *
 *   const t = useT();
 *   t("titleBar.open");                       // "Open"
 *   t("statusBar.bookmarks", { n: 42 });      // "42 bookmarks"
 *
 * Unknown keys return the key itself so a missing translation degrades
 * gracefully and is easy to spot in the UI.
 */

import { createContext, useContext, useMemo, type ReactNode } from "react";

import { locales } from "./locales";
import type { Locale, TranslationKey, Translations } from "./types";

interface I18nValue {
  locale: Locale;
  /** Resolve a key to a string, optionally substituting `{name}` placeholders. */
  t: (
    key: TranslationKey | keyof Translations,
    params?: Record<string, string | number>,
  ) => string;
}

const I18nContext = createContext<I18nValue | null>(null);

export function I18nProvider({
  locale = "en",
  children,
}: {
  locale?: Locale;
  children: ReactNode;
}) {
  const value = useMemo<I18nValue>(() => {
    const dict = locales[locale];
    return {
      locale,
      t: (key, params) => {
        const raw = dict[key as string] ?? (key as string);
        if (!params) return raw;
        return Object.entries(params).reduce(
          (acc, [k, v]) =>
            acc.replace(new RegExp(`\\{${k}\\}`, "g"), String(v)),
          raw,
        );
      },
    };
  }, [locale]);

  return (
    <I18nContext.Provider value={value}>{children}</I18nContext.Provider>
  );
}

/** Hook returning the translator function. */
export function useT(): I18nValue["t"] {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useT must be called inside <I18nProvider>");
  return ctx.t;
}

/** Hook returning the active locale code (useful for `lang=` attributes). */
export function useLocale(): Locale {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useLocale must be called inside <I18nProvider>");
  return ctx.locale;
}
