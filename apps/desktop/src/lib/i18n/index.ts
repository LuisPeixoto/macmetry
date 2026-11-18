// Translation without a library: a flat dictionary per language, `{name}` placeholders, English as the fallback.

export type Language = "en" | "pt-BR";
export type Dict = Record<string, string>;

/** Picks the language to show from the setting and the browser's preferences. Anything that is not Portuguese is English. */
export function resolveLanguage(setting: "system" | Language, preferred: readonly string[]): Language {
  if (setting !== "system") return setting;
  return preferred.some((l) => l.toLowerCase().startsWith("pt")) ? "pt-BR" : "en";
}

/** Looks a key up and fills in `{placeholders}`. A key missing from `dict` falls back to English; missing there too, it shows the key itself so the gap is visible. */
export function translate(dict: Dict, fallback: Dict, key: string, params?: Record<string, string | number>): string {
  const template = dict[key] ?? fallback[key] ?? key;
  return params ? template.replace(/\{(\w+)\}/g, (whole, name: string) => (name in params ? String(params[name]) : whole)) : template;
}
