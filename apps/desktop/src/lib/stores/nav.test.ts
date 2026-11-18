import { test } from "node:test";
import assert from "node:assert/strict";
import { availableViews, NAV } from "./nav.ts";
import { resolveTheme } from "./theme.ts";
import type { Capabilities } from "../types.ts";

const caps = (over: Partial<Capabilities> = {}): Capabilities => ({
  cpu: true, memory: true, disks: true, network: true, gpu: true, temperatures: true, fans: true, power: true,
  battery: true, thermal_pressure: true, processes: true, ...over,
});

test("a desktop has no battery view, and every Mac has the rest", () => {
  const ids = (c: Capabilities) => availableViews(c).map((n) => n.id);
  assert.ok(!ids(caps({ battery: false })).includes("battery"));
  assert.ok(ids(caps()).includes("battery"));
  assert.deepEqual(ids(caps()), NAV.map((n) => n.id));
});

test("sensors show when there is a temperature, a fan or a power reading", () => {
  const ids = (c: Capabilities) => availableViews(c).map((n) => n.id);
  assert.ok(!ids(caps({ temperatures: false, fans: false, power: false })).includes("sensors"));
  assert.ok(ids(caps({ temperatures: false, fans: false, power: true })).includes("sensors"));
  assert.ok(!ids(caps({ gpu: false })).includes("gpu"));
});

test("before the Mac is known every view is listed", () => {
  assert.equal(availableViews(null).length, NAV.length);
});

test("the theme follows the setting, or the system when asked", () => {
  assert.equal(resolveTheme("dark", false), "dark");
  assert.equal(resolveTheme("light", true), "light");
  assert.equal(resolveTheme("system", true), "dark");
  assert.equal(resolveTheme("system", false), "light");
});
