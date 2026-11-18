import type { Settings } from "../types.ts";

/** `dark` or `light`: the setting, or the system's when it says "system". */
export function resolveTheme(setting: Settings["theme"], systemPrefersDark: boolean): "dark" | "light" {
  return setting === "system" ? (systemPrefersDark ? "dark" : "light") : setting;
}
