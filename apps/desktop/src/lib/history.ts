// Turning what the history database returns into what the chart draws. Pure, so it is tested.

import type { Series, Unit } from "./types.ts";
import type { Value } from "./chart.ts";

export const RANGES = { "1h": 3600_000, "6h": 6 * 3600_000, "24h": 24 * 3600_000, "7d": 7 * 86400_000, "30d": 30 * 86400_000 } as const;
export type Range = keyof typeof RANGES;

export interface Aligned {
  times: number[];
  /** One entry per input series, in order, with `null` where that metric had no point at that time. */
  avg: Value[][];
  min: Value[][];
  max: Value[][];
  resolutionMs: number;
}

/** Puts several series on one shared time axis. A metric without a point at a time gets `null`, which breaks its line. */
export function align(series: Series[]): Aligned {
  const times = [...new Set(series.flatMap((s) => s.points.map((p) => p.ts_ms)))].sort((a, b) => a - b);
  const column = (pick: "avg" | "min" | "max") =>
    series.map((s) => {
      const byTime = new Map(s.points.map((p) => [p.ts_ms, p[pick]] as const));
      return times.map((t) => byTime.get(t) ?? null);
    });
  return { times, avg: column("avg"), min: column("min"), max: column("max"), resolutionMs: Math.max(1000, ...series.map((s) => s.resolution_ms)) };
}

/**
 * Groups metrics by unit. Two units on one chart would need two y axes, which makes it too easy to read a false
 * relationship off the picture, so each unit gets its own chart.
 */
export function groupByUnit<T extends { key: string; unit: Unit }>(metrics: T[]): { unit: Unit; metrics: T[] }[] {
  const groups: { unit: Unit; metrics: T[] }[] = [];
  for (const m of metrics) {
    const group = groups.find((g) => g.unit === m.unit);
    if (group) group.metrics.push(m);
    else groups.push({ unit: m.unit, metrics: [m] });
  }
  return groups;
}

/** The most points one chart asks for: more than a wide screen can show would only be thrown away. */
export const MAX_POINTS = 480;

/** How often to look again, in milliseconds: often enough to feel live for a short range, rare for a long one. */
export const refreshEvery = (rangeMs: number): number => (rangeMs <= RANGES["1h"] ? 5_000 : rangeMs <= RANGES["24h"] ? 15_000 : 60_000);
