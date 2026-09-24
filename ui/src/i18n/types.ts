/**
 * Localisation types.
 *
 * The translation table is intentionally a flat `Record<string, string>`
 * keyed by dot-separated paths.  This keeps the shape the same across
 * locales (so a missing key in fr.ts shows up as a TypeScript-friendly
 * subset of `en.ts` keys) and avoids a nested-object lookup at call time.
 *
 * v0.0.9 ships English only.  Adding a locale is a new file under
 * `ui/src/i18n/<code>.ts` registered in `locales.ts`.  No build-step
 * changes are required.
 */

export type Locale = "en";

/** Dot-separated translation key, e.g. `"titleBar.tools.diff"`. */
export type TranslationKey = string;

/** Flat object: keys are dot-paths, values are the displayed string. */
export type Translations = Record<TranslationKey, string>;
