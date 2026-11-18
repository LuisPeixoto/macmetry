// The formatters bound to the language in use. A template calls `fmt.bytes(n)` and it follows the setting.

import * as f from "../format.ts";
import { i18n } from "./i18n.svelte.ts";
import type { Unit } from "../types.ts";

export const fmt = {
  num: (v: number | null | undefined, digits = 0) => f.num(v, digits, i18n.locale),
  percent: (v: number | null | undefined, digits = 0) => f.percent(v, digits, i18n.locale),
  bytes: (v: number | null | undefined) => f.bytes(v, i18n.locale, "binary"),
  /** For disks and volumes, which macOS counts in 1000s. */
  disk: (v: number | null | undefined) => f.bytes(v, i18n.locale, "decimal"),
  rate: (v: number | null | undefined) => f.rate(v, i18n.locale),
  temp: (v: number | null | undefined) => f.temperature(v, i18n.locale),
  watts: (v: number | null | undefined) => f.watts(v, i18n.locale),
  rpm: (v: number | null | undefined) => f.rpm(v, i18n.locale),
  freq: (v: number | null | undefined) => f.frequency(v, i18n.locale),
  duration: f.duration,
  minutes: f.minutes,
  value: (unit: Unit, v: number | null | undefined) => f.value(unit, v, i18n.locale),
  clock: (ms: number, seconds = false) => f.clock(ms, seconds, i18n.locale),
  dateTime: (ms: number) => f.dateTime(ms, i18n.locale),
};
