import { test } from "node:test";
import assert from "node:assert/strict";
import { align, groupByUnit, refreshEvery, RANGES } from "./history.ts";
import type { Series } from "./types.ts";

const s = (metric: string, points: [number, number][], res = 1000): Series => ({
  metric, resolution_ms: res, points: points.map(([ts_ms, avg]) => ({ ts_ms, avg, min: avg - 1, max: avg + 1 })),
});

test("series share one time axis, with gaps where a metric has nothing", () => {
  const a = align([s("cpu.total", [[1000, 10], [2000, 20], [3000, 30]]), s("temp.max", [[2000, 60]])]);
  assert.deepEqual(a.times, [1000, 2000, 3000]);
  assert.deepEqual(a.avg[0], [10, 20, 30]);
  assert.deepEqual(a.avg[1], [null, 60, null], "a missing point is null, never zero");
  assert.deepEqual(a.min[1], [null, 59, null]);
  assert.deepEqual(a.max[0], [11, 21, 31]);
});

test("the axis is sorted and has no repeats even when series interleave", () => {
  const a = align([s("a", [[3000, 1], [1000, 1]]), s("b", [[2000, 1], [3000, 1]])]);
  assert.deepEqual(a.times, [1000, 2000, 3000]);
});

test("nothing in, nothing out", () => {
  const a = align([]);
  assert.deepEqual([a.times, a.avg], [[], []]);
  assert.equal(align([s("a", [])]).times.length, 0);
});

test("the resolution is the coarsest of the series", () => {
  assert.equal(align([s("a", [[0, 1]], 1000), s("b", [[0, 1]], 60_000)]).resolutionMs, 60_000);
  assert.equal(align([]).resolutionMs, 1000);
});

test("metrics with different units never share a chart", () => {
  const groups = groupByUnit([
    { key: "cpu.total", unit: "percent" as const }, { key: "temp.max", unit: "celsius" as const }, { key: "memory.used_pct", unit: "percent" as const },
  ]);
  assert.deepEqual(groups.map((g) => [g.unit, g.metrics.map((m) => m.key)]), [["percent", ["cpu.total", "memory.used_pct"]], ["celsius", ["temp.max"]]]);
  assert.deepEqual(groupByUnit([]), []);
});

test("a short range refreshes often and a long one rarely", () => {
  assert.ok(refreshEvery(RANGES["1h"]) < refreshEvery(RANGES["24h"]));
  assert.ok(refreshEvery(RANGES["24h"]) < refreshEvery(RANGES["30d"]));
});
