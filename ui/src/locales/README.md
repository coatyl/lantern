# Lantern locales

A new locale is a new file `<code>.ts` in `ui/src/i18n/` that exports a
`Translations` object whose keys mirror those in `en.ts` exactly, plus a
one-line registration in `ui/src/i18n/locales.ts` (and a corresponding
widening of the `Locale` union in `ui/src/i18n/types.ts`). No build-step
changes, no extra tooling, no Rust changes; drop the file in via PR and
the existing `useT()` resolution picks it up. Future contributors are
welcome to land a French, German, Japanese, etc. file the same way.
