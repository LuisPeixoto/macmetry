import { test } from "node:test";
import assert from "node:assert/strict";
import { assignColors, MAX_SERIES } from "./colors.ts";
import { METRICS } from "./metrics.ts";

test("every metric gets a colour", () => {
  for (const m of METRICS) assert.match(assignColors([m.key])[m.key], /^var\(--s[1-8]\)$/);
});

test("two lines on a chart never share a colour", () => {
  // These two both prefer slot 3 or 2 in places; the later one moves to a free slot.
  const colors = assignColors(["cpu.user", "gpu.utilization", "battery.percent", "power.gpu"]);
  assert.equal(new Set(Object.values(colors)).size, 4, JSON.stringify(colors));
});

test("a metric keeps its colour whichever other metrics are on the chart", () => {
  const alone = assignColors(["cpu.total"])["cpu.total"];
  assert.equal(assignColors(["cpu.total", "memory.used_pct"])["cpu.total"], alone, "CPU stays blue");
  assert.equal(assignColors(["memory.used_pct", "cpu.total"])["cpu.total"], alone, "whatever order they were ticked in");
});

test("the palette has eight slots, and eight metrics fit", () => {
  const keys = METRICS.slice(0, MAX_SERIES).map((m) => m.key);
  assert.equal(new Set(Object.values(assignColors(keys))).size, MAX_SERIES);
});
