// Numbers for people. Pure functions that take the locale, so they are tested in English and Portuguese and the
// components never format by hand.

import type { Unit } from "./types.ts";

export type Locale = "en" | "pt-BR";

const cache = new Map<string, Intl.NumberFormat>();
function nf(locale: Locale, digits: number): Intl.NumberFormat {
  const key = `${locale}:${digits}`;
  let f = cache.get(key);
  if (!f) cache.set(key, (f = new Intl.NumberFormat(locale, { minimumFractionDigits: digits, maximumFractionDigits: digits })));
  return f;
}

/** A number with a fixed count of decimals and the locale's separators. Non-finite numbers show as a dash. */
export function num(v: number | null | undefined, digits = 0, locale: Locale = "en"): string {
  return typeof v === "number" && Number.isFinite(v) ? nf(locale, digits).format(v) : "–";
}

export const percent = (v: number | null | undefined, digits = 0, locale: Locale = "en") =>
  typeof v === "number" && Number.isFinite(v) ? `${num(v, digits, locale)}%` : "–";

const BINARY = ["B", "KB", "MB", "GB", "TB", "PB"];

/**
 * A size. `binary` counts in 1024s, as Activity Monitor does for memory (32 GiB shows as "32 GB"); `decimal` counts
 * in 1000s, as Finder does for disks (a 512 GB drive shows as "512 GB").
 */
export function bytes(v: number | null | undefined, locale: Locale = "en", mode: "binary" | "decimal" = "binary"): string {
  if (typeof v !== "number" || !Number.isFinite(v)) return "–";
  const base = mode === "binary" ? 1024 : 1000;
  let value = Math.max(0, v);
  let unit = 0;
  while (value >= base && unit < BINARY.length - 1) {
    value /= base;
    unit++;
  }
  // Whole bytes need no decimals, and big numbers read better with fewer.
  const digits = unit === 0 ? 0 : value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${num(value, digits, locale)} ${BINARY[unit]}`;
}

export const rate = (v: number | null | undefined, locale: Locale = "en") =>
  typeof v === "number" && Number.isFinite(v) ? `${bytes(v, locale, "decimal")}/s` : "–";

export const temperature = (v: number | null | undefined, locale: Locale = "en") =>
  typeof v === "number" && Number.isFinite(v) ? `${num(v, 0, locale)} °C` : "–";

export const watts = (v: number | null | undefined, locale: Locale = "en") =>
  typeof v === "number" && Number.isFinite(v) ? `${num(v, Math.abs(v) < 10 ? 1 : 0, locale)} W` : "–";

export const rpm = (v: number | null | undefined, locale: Locale = "en") =>
  typeof v === "number" && Number.isFinite(v) ? `${num(v, 0, locale)} rpm` : "–";

/** A clock speed: 3400 becomes "3.4 GHz", 800 stays "800 MHz". */
export function frequency(mhz: number | null | undefined, locale: Locale = "en"): string {
  if (typeof mhz !== "number" || !Number.isFinite(mhz)) return "–";
  return mhz >= 1000 ? `${num(mhz / 1000, 2, locale)} GHz` : `${num(mhz, 0, locale)} MHz`;
}

/** Seconds as the two largest units: `45s`, `3m 05s`, `2h 05m`, `3d 04h`. */
export function duration(seconds: number | null | undefined): string {
  if (typeof seconds !== "number" || !Number.isFinite(seconds) || seconds < 0) return "–";
  const s = Math.floor(seconds);
  const [d, h, m, sec] = [Math.floor(s / 86400), Math.floor(s / 3600) % 24, Math.floor(s / 60) % 60, s % 60];
  const pad = (n: number) => String(n).padStart(2, "0");
  if (d > 0) return `${d}d ${pad(h)}h`;
  if (h > 0) return `${h}h ${pad(m)}m`;
  if (m > 0) return `${m}m ${pad(sec)}s`;
  return `${sec}s`;
}

/** Minutes of battery as `4 h 43 min`. */
export function minutes(total: number | null | undefined): string {
  if (typeof total !== "number" || !Number.isFinite(total) || total < 0) return "–";
  const t = Math.round(total);
  return t >= 60 ? `${Math.floor(t / 60)} h ${String(t % 60).padStart(2, "0")} min` : `${t} min`;
}

/** A value with the unit of its metric. Levels (pressure) are named by the caller, so they show as numbers here. */
export function value(unit: Unit, v: number | null | undefined, locale: Locale = "en"): string {
  switch (unit) {
    case "percent": return percent(v, 0, locale);
    case "bytes": return bytes(v, locale);
    case "bytes_per_second": return rate(v, locale);
    case "celsius": return temperature(v, locale);
    case "rpm": return rpm(v, locale);
    case "watts": return watts(v, locale);
    case "per_second": return typeof v === "number" ? `${num(v, 0, locale)}/s` : "–";
    case "level": return num(v, 2, locale);
  }
}

/** A short clock time for a chart axis: `14:05`, or `14:05:30` when the points are seconds apart. */
export function clock(ms: number, withSeconds = false, locale: Locale = "en"): string {
  return new Date(ms).toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit", ...(withSeconds ? { second: "2-digit" } : {}), hour12: false });
}

/** A date and time for a tooltip over a long range. */
export function dateTime(ms: number, locale: Locale = "en"): string {
  return new Date(ms).toLocaleString(locale, { day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit", hour12: false });
}

/**
 * A readable name for the processor. Apple chips already read well ("Apple M3 Pro"); an Intel brand string carries
 * trademarks and the clock ("Intel(R) Core(TM) i9-9980HK CPU @ 2.40GHz"), which are trimmed. Some Macs report a
 * placeholder like "0000" instead of a model, and then the core count says more.
 */
export function chipLabel(chip: string, physicalCores = 0): string {
  const clean = chip.replace(/\((R|TM)\)/gi, "").replace(/\bCPU\b/gi, "").replace(/@.*$/, "").replace(/\bGenuine\b/gi, "").replace(/\s+/g, " ").trim();
  if (!clean || /\b0000\b/.test(clean)) return physicalCores > 0 ? `Intel · ${physicalCores} cores` : "Intel";
  return clean;
}
