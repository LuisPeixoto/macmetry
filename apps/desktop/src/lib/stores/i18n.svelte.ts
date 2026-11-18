import { resolveLanguage, translate, type Language } from "../i18n/index.ts";
import en from "../i18n/en.ts";
import ptBR from "../i18n/pt-BR.ts";

const dictionaries = { en, "pt-BR": ptBR } as const;

class I18n {
  setting = $state<"system" | Language>("system");
  /** The language in use: the setting, or the system's when it says "system". */
  lang = $derived<Language>(resolveLanguage(this.setting, typeof navigator === "undefined" ? [] : navigator.languages));
  locale = $derived(this.lang);

  t = (key: string, params?: Record<string, string | number>): string => translate(dictionaries[this.lang], en, key, params);
}

export const i18n = new I18n();
/** Translate a key in the current language. Reads reactive state, so a template that calls it updates on a change. */
export const t = i18n.t;
