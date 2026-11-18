// The arithmetic behind the charts, kept apart from the drawing so it can be tested.

export type Value = number | null;

/** The smallest and largest real value across several series, or `null` when there is none. */
export function extent(series: Value[][]): [number, number] | null {
  let lo = Infinity;
  let hi = -Infinity;
  for (const values of series) for (const v of values) if (typeof v === "number" && Number.isFinite(v)) { lo = Math.min(lo, v); hi = Math.max(hi, v); }
  return lo <= hi ? [lo, hi] : null;
}

/** A step of 1, 2, 2.5 or 5 times a power of ten that gives about `count` intervals over `span`. (2.5 makes 0 to 100 read as quarters.) */
export function niceStep(span: number, count: number): number {
  if (!(span > 0) || count < 1) return 1;
  const raw = span / count;
  const magnitude = 10 ** Math.floor(Math.log10(raw));
  const f = raw / magnitude;
  return (f < 1.5 ? 1 : f < 2.25 ? 2 : f < 3.5 ? 2.5 : f < 7.5 ? 5 : 10) * magnitude;
}

/** Axis ticks that start at or below `min` and end at or above `max`, on round numbers. */
export function niceTicks(min: number, max: number, count = 4): number[] {
  if (!Number.isFinite(min) || !Number.isFinite(max)) return [0, 1];
  if (max <= min) max = min + 1;
  const step = niceStep(max - min, count);
  const first = Math.floor(min / step) * step;
  const ticks: number[] = [];
  for (let v = first; v < max + step * 0.999; v += step) ticks.push(Math.round(v / step) * step);
  return ticks;
}

/**
 * Axis ticks for a size or a speed. They are chosen in the unit the label will use (megabytes, say), so the axis reads
 * 0, 50 MB, 100 MB instead of 0, 47.7 MB, 95.4 MB. `base` is 1024 for memory and 1000 for disks and networks.
 */
export function byteTicks(max: number, base: 1000 | 1024, count = 4): number[] {
  if (!Number.isFinite(max) || max <= 0) return [0, 1];
  const scale = base ** Math.min(5, Math.max(0, Math.floor(Math.log(max) / Math.log(base))));
  return niceTicks(0, max / scale, count).map((v) => v * scale);
}

/**
 * The y range of a chart. A percentage chart always shows 0 to 100 so a quiet Mac does not look busy; anything else
 * starts at 0 and grows to a round number just above the data, so the chart never touches its own top.
 */
export function yRange(data: [number, number] | null, unit: "percent" | "other", pinned?: { min?: number; max?: number }): [number, number] {
  if (unit === "percent") return [pinned?.min ?? 0, pinned?.max ?? 100];
  const hi = data ? data[1] : 1;
  const top = pinned?.max ?? niceTicks(0, hi <= 0 ? 1 : hi * 1.1, 4).at(-1)!;
  return [pinned?.min ?? 0, top];
}

/**
 * An SVG path through the points. A missing value (`null`) breaks the line instead of being drawn as zero, so a Mac
 * that stopped reporting something shows a gap.
 */
export function linePath(values: Value[], x: (i: number) => number, y: (v: number) => number): string {
  let d = "";
  let pen = false;
  values.forEach((v, i) => {
    if (typeof v !== "number" || !Number.isFinite(v)) { pen = false; return; }
    d += `${pen ? "L" : "M"}${x(i).toFixed(1)},${y(v).toFixed(1)}`;
    pen = true;
  });
  return d;
}

/** The area under the line down to `baseline`, one closed shape per unbroken run. */
export function areaPath(values: Value[], x: (i: number) => number, y: (v: number) => number, baseline: number): string {
  let d = "";
  let start = -1;
  const close = (end: number) => {
    if (start >= 0) d += `L${x(end).toFixed(1)},${baseline.toFixed(1)}L${x(start).toFixed(1)},${baseline.toFixed(1)}Z`;
    start = -1;
  };
  values.forEach((v, i) => {
    if (typeof v !== "number" || !Number.isFinite(v)) { close(i - 1); return; }
    d += `${start < 0 ? "M" : "L"}${x(i).toFixed(1)},${y(v).toFixed(1)}`;
    if (start < 0) start = i;
  });
  close(values.length - 1);
  return d;
}

/** Which point is nearest to a pointer at `px` when `n` points are spread across `width` starting at `left`. */
export function nearestIndex(px: number, left: number, width: number, n: number): number {
  if (n <= 1 || width <= 0) return 0;
  return Math.min(n - 1, Math.max(0, Math.round(((px - left) / width) * (n - 1))));
}

/** The last real value of a series, for the legend and the big number. */
export function lastValue(values: Value[]): number | null {
  for (let i = values.length - 1; i >= 0; i--) { const v = values[i]; if (typeof v === "number" && Number.isFinite(v)) return v; }
  return null;
}

/** A colour for a load: calm, busy, hot. Thresholds are the same everywhere so a colour means one thing. */
export function loadLevel(percent: number | null): "ok" | "busy" | "hot" {
  if (percent === null) return "ok";
  return percent >= 90 ? "hot" : percent >= 70 ? "busy" : "ok";
}

/**
 * The band between a lowest and a highest value, one closed shape per unbroken run. Drawn faintly behind the line, it
 * shows the spikes that averaging a long range would otherwise hide.
 */
export function bandPath(min: Value[], max: Value[], x: (i: number) => number, y: (v: number) => number): string {
  const ok = (i: number) => typeof min[i] === "number" && typeof max[i] === "number" && Number.isFinite(min[i]) && Number.isFinite(max[i]);
  let d = "";
  let i = 0;
  while (i < max.length) {
    if (!ok(i)) { i++; continue; }
    let end = i;
    while (end + 1 < max.length && ok(end + 1)) end++;
    const run = Array.from({ length: end - i + 1 }, (_, k) => i + k);
    d += `M${x(i).toFixed(1)},${y(max[i] as number).toFixed(1)}`;
    for (const k of run.slice(1)) d += `L${x(k).toFixed(1)},${y(max[k] as number).toFixed(1)}`;
    for (const k of [...run].reverse()) d += `L${x(k).toFixed(1)},${y(min[k] as number).toFixed(1)}`;
    d += "Z";
    i = end + 1;
  }
  return d;
}
