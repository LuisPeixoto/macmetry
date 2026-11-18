import { test } from "node:test";
import assert from "node:assert/strict";
import { areaPath, bandPath, byteTicks, extent, lastValue, linePath, loadLevel, nearestIndex, niceStep, niceTicks, yRange } from "./chart.ts";

test("ticks fall on round numbers and cover the data", () => {
  assert.deepEqual(niceTicks(0, 100, 4), [0, 25, 50, 75, 100], "0..100 in quarters");
  assert.deepEqual(niceTicks(0, 7.3, 4), [0, 2, 4, 6, 8]);
  const t = niceTicks(3, 97, 4);
  assert.ok(t[0] <= 3 && t.at(-1)! >= 97);
  const flat = niceTicks(5, 5);
  assert.ok(flat[0] === 5 && flat.at(-1) === 6 && flat.length >= 2, "no span still gives a usable axis");
  assert.deepEqual(niceTicks(Number.NaN, 1), [0, 1]);
});

test("the step is 1, 2, 2.5 or 5 times a power of ten", () => {
  for (const span of [0.37, 1, 9, 73, 1234, 98765]) {
    const step = niceStep(span, 4);
    const mantissa = step / 10 ** Math.floor(Math.log10(step));
    assert.ok([1, 2, 2.5, 5, 10].includes(Math.round(mantissa * 1000) / 1000), `${span} -> ${step}`);
  }
  assert.equal(niceStep(0, 4), 1);
});

test("a percentage chart always spans 0 to 100", () => {
  assert.deepEqual(yRange([2, 9], "percent"), [0, 100]);
  assert.deepEqual(yRange(null, "percent"), [0, 100]);
});

test("other charts start at zero and leave room above the data", () => {
  const [lo, hi] = yRange([0, 7_300_000], "other");
  assert.equal(lo, 0);
  assert.ok(hi >= 7_300_000 && hi <= 12_000_000, String(hi));
  assert.equal(yRange(null, "other")[0], 0, "no data still gives an axis");
  assert.equal(yRange([0, 5], "other", { max: 100 })[1], 100, "a pinned maximum wins");
});

test("finds the extent and ignores gaps", () => {
  assert.deepEqual(extent([[1, null, 5], [3, 9, null]]), [1, 9]);
  assert.equal(extent([[null, null]]), null);
  assert.equal(extent([]), null);
  assert.deepEqual(extent([[Number.NaN, 2]]), [2, 2]);
});

test("a missing value breaks the line instead of dropping to zero", () => {
  const x = (i: number) => i * 10;
  const y = (v: number) => 100 - v;
  assert.equal(linePath([10, 20, null, 30, 40], x, y), "M0.0,90.0L10.0,80.0M30.0,70.0L40.0,60.0");
  assert.equal(linePath([], x, y), "");
  assert.equal(linePath([null, null], x, y), "");
});

test("an area is closed once per unbroken run", () => {
  const x = (i: number) => i * 10;
  const y = (v: number) => 100 - v;
  const d = areaPath([10, 20, null, 30], x, y, 100);
  assert.equal((d.match(/Z/g) ?? []).length, 2);
  assert.ok(d.startsWith("M0.0,90.0L10.0,80.0L10.0,100.0L0.0,100.0Z"));
  assert.equal(areaPath([null], x, y, 100), "");
});

test("the pointer snaps to the nearest point and stays inside", () => {
  assert.equal(nearestIndex(0, 0, 100, 11), 0);
  assert.equal(nearestIndex(100, 0, 100, 11), 10);
  assert.equal(nearestIndex(54, 0, 100, 11), 5);
  assert.equal(nearestIndex(-50, 0, 100, 11), 0);
  assert.equal(nearestIndex(500, 0, 100, 11), 10);
  assert.equal(nearestIndex(50, 0, 100, 1), 0);
  assert.equal(nearestIndex(50, 0, 0, 5), 0);
});

test("the last real value is the latest one that exists", () => {
  assert.equal(lastValue([1, 2, null, null]), 2);
  assert.equal(lastValue([null]), null);
  assert.equal(lastValue([]), null);
});

test("load levels share thresholds", () => {
  assert.deepEqual([null, 10, 70, 89, 90, 100].map(loadLevel), ["ok", "ok", "busy", "busy", "hot", "hot"]);
});

test("size and speed axes land on round numbers of their own unit", () => {
  // 238 MB of data: the old axis said 47.7 MB, 95.4 MB ...
  const mb = 1024 ** 2;
  assert.deepEqual(byteTicks(238 * mb, 1024).map((v) => v / mb), [0, 50, 100, 150, 200, 250], "whole multiples of 50 MB");
  assert.deepEqual(byteTicks(7_300_000, 1000).map((v) => v / 1e6), [0, 2, 4, 6, 8]);
  assert.deepEqual(byteTicks(0, 1000), [0, 1]);
  assert.deepEqual(byteTicks(Number.NaN, 1024), [0, 1]);
  assert.ok(byteTicks(3, 1024).at(-1)! >= 3, "a few bytes still gets an axis");
});

test("a band runs along the highest values and back along the lowest, once per unbroken run", () => {
  const x = (i: number) => i * 10;
  const y = (v: number) => 100 - v;
  const d = bandPath([10, 20, null, 30], [15, 25, null, 35], x, y);
  assert.equal(d, "M0.0,85.0L10.0,75.0L10.0,80.0L0.0,90.0ZM30.0,65.0L30.0,70.0Z");
  assert.equal(bandPath([], [], x, y), "");
  assert.equal(bandPath([null], [5], x, y), "", "a point with only one end is not drawn");
});
