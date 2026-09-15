/**
 * Locale registry: maps every supported `Locale` code to its translation
 * table.  v0.0.9 ships English only.
 *
 * To add a new locale: create `ui/src/i18n/<code>.ts` exporting a
 * `Translations` object, widen the `Locale` union in `./types`, and
 * register it here.  No other code changes are required.
 */

import { en } from "./en";
import type { Locale, Translations } from "./types";

export const locales: Record<Locale, Translations> = { en };
