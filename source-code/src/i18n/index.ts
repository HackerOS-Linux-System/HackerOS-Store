import en, { type TranslationKey } from "./en";
import pl from "./pl";

export type Lang = "en" | "pl";
export type { TranslationKey };

export const LANGUAGES: { id: Lang; label: string }[] = [
  { id: "en", label: "English" },
  { id: "pl", label: "Polski" },
];

export const LOCALES: Record<Lang, Record<TranslationKey, string>> = {
  en,
  pl,
};

const FALLBACK: Lang = "en";

export function translate(lang: Lang, key: TranslationKey | string): string {
  const table = LOCALES[lang] ?? LOCALES[FALLBACK];
  const fallbackTable = LOCALES[FALLBACK];
  return (table as Record<string, string>)[key]
    ?? (fallbackTable as Record<string, string>)[key]
    ?? key;
}

/**
 * `t("a11y.starRating", { n: 4 })` -> replaces `{n}` in the string.
 * Only used by a couple of accessibility labels; kept intentionally tiny
 * (no plural rules, no ICU) since the app's needs are simple.
 */
export function format(template: string, vars?: Record<string, string | number>): string {
  if (!vars) return template;
  return Object.entries(vars).reduce(
    (acc, [k, v]) => acc.split(`{${k}}`).join(String(v)),
    template,
  );
}
