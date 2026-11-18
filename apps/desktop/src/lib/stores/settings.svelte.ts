// The person's choices. The Rust side is the source of truth; this mirrors it and applies what the page itself shows
// (language and theme).

import { getBackend, inApp } from "../api.ts";
import { defaultSettings } from "../defaults.ts";
import type { Settings } from "../types.ts";
import { i18n } from "./i18n.svelte.ts";
import { resolveTheme } from "./theme.ts";

const THEME_KEY = "macmetry.theme";

class SettingsStore {
  value = $state.raw<Settings>(defaultSettings());
  /** The theme in effect, after resolving "system". */
  theme = $state<"dark" | "light">("dark");
  loaded = $state(false);
  private media: MediaQueryList | null = null;

  async load() {
    const backend = await getBackend();
    this.value = await backend.getSettings();
    this.loaded = true;
    this.apply();
    this.media ??= globalThis.matchMedia?.("(prefers-color-scheme: dark)") ?? null;
    this.media?.addEventListener("change", () => this.apply());
  }

  /** Saves a change and applies it. The Rust side validates and may refuse, in which case the error is thrown. */
  async update(patch: Partial<Settings>) {
    const backend = await getBackend();
    this.value = await backend.setSettings({ ...this.value, ...patch });
    this.apply();
  }

  private apply() {
    i18n.setting = this.value.language;
    this.theme = resolveTheme(this.value.theme, this.media?.matches ?? true);
    // In a plain browser `?theme=` (see public/early.js) is for design work and wins over the setting.
    const forced = inApp ? null : new URLSearchParams(location.search).get("theme");
    if (forced === "dark" || forced === "light") this.theme = forced;
    document.documentElement.dataset.theme = this.theme;
    // The early script in the page reads this to pick the theme before anything has loaded, so there is no flash.
    try { localStorage.setItem(THEME_KEY, this.value.theme); } catch { /* storage can be unavailable */ }
  }
}

export const settings = new SettingsStore();
