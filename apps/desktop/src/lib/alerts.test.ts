import { test } from "node:test";
import assert from "node:assert/strict";
import { COOLDOWN_OPTIONS, displayName, durationLabel, FOR_OPTIONS, fieldFor, fromInput, levelToThreshold, newRule, problem, thresholdToLevel, toInput } from "./alerts.ts";
import { METRICS } from "./metrics.ts";
import { defaultSettings } from "./defaults.ts";

const metric = (key: string) => METRICS.find((m) => m.key === key)!;

test("every metric has a field to type a threshold into", () => {
  for (const m of METRICS) {
    const f = fieldFor(m);
    assert.ok(f.factor > 0 && f.step > 0, m.key);
  }
});

test("pressure metrics are chosen from named levels; the rest are numbers", () => {
  assert.equal(fieldFor(metric("memory.pressure")).kind, "level");
  assert.equal(fieldFor(metric("thermal.pressure")).kind, "level");
  assert.equal(fieldFor(metric("cpu.total")).kind, "number");
  assert.equal(fieldFor(metric("cpu.load1")).kind, "number", "a load average is a plain number, not a level name");
});

test("sizes are typed in friendly units and stored in bytes", () => {
  const swap = fieldFor(metric("memory.swap_used"));
  assert.equal(swap.suffix, "GB");
  assert.equal(fromInput(2, swap), 2 * 1024 ** 3);
  assert.equal(toInput(2 * 1024 ** 3, swap), 2);
  const net = fieldFor(metric("net.rx_bps"));
  assert.equal(fromInput(50, net), 50_000_000);
  assert.equal(toInput(50_000_000, net), 50);
  assert.equal(toInput(90, fieldFor(metric("cpu.total"))), 90, "percent needs no conversion");
});

test("a new rule gets a usable id, a sensible start and is on", () => {
  const first = newRule(metric("temp.max"), []);
  assert.deepEqual([first.id, first.metric, first.comparator, first.threshold, first.enabled], ["custom_1", "temp.max", "above", 85, true]);
  const second = newRule(metric("cpu.total"), [first]);
  assert.equal(second.id, "custom_2");
  const skipped = newRule(metric("cpu.total"), [{ ...first, id: "custom_1" }, { ...first, id: "custom_2" }]);
  assert.equal(skipped.id, "custom_3");
  assert.ok(newRule(metric("memory.swap_used"), []).threshold > 0);
});

test("a rule needs a name and a real threshold", () => {
  const rule = newRule(metric("cpu.total"), []);
  assert.equal(problem(rule), null);
  assert.equal(problem({ ...rule, name: "  " }), "name");
  assert.equal(problem({ ...rule, threshold: Number.NaN }), "threshold");
});

test("the offered durations are sorted and readable", () => {
  assert.deepEqual([...FOR_OPTIONS], [...FOR_OPTIONS].sort((a, b) => a - b));
  assert.deepEqual([...COOLDOWN_OPTIONS], [...COOLDOWN_OPTIONS].sort((a, b) => a - b));
  assert.deepEqual([0, 30, 120, 3600, 21600].map(durationLabel), ["0 s", "30 s", "2 min", "1 h", "6 h"]);
});

test("a level alert fires when the level is reached", () => {
  assert.equal(levelToThreshold(2), 1.5, "critical memory pressure is level 2, stored as above 1.5");
  assert.equal(thresholdToLevel(1.5), 2);
  assert.equal(thresholdToLevel(0.5), 1);
  for (const level of [1, 2, 3, 4]) assert.equal(thresholdToLevel(levelToThreshold(level)), level);
});

test("a suggested alert is shown in the language in use until it is renamed", () => {
  const original = defaultSettings().alerts;
  const pt = (id: string) => `pt:${id}`;
  assert.equal(displayName(original[0], original, pt), "pt:disk_low");
  assert.equal(displayName({ ...original[0], name: "My disk alert" }, original, pt), "My disk alert", "a name the person chose is kept");
  const custom = newRule(metric("cpu.total"), original);
  assert.equal(displayName(custom, original, pt), custom.name, "a rule that is not a suggestion is never translated");
});
