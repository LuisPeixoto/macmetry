import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { METRICS, metricValue } from "./metrics.ts";
import type { Snapshot } from "./types.ts";

const empty: Snapshot = {
  timestamp_ms: 0, elapsed_ms: 0, thermal_pressure: null, cpu: null, memory: null, disks: null, network: null,
  gpus: [], sensors: null, power: null, battery: null, processes: [],
};

test("the metric list matches the Rust one, key for key, in order", () => {
  const rust = readFileSync(fileURLToPath(new URL("../../../../crates/macmetry-core/src/metric.rs", import.meta.url)), "utf8");
  const unit = (u: string) => u.replace(/([a-z])([A-Z])/g, "$1_$2").toLowerCase();
  const rows = [...rust.matchAll(/^\s+\w+ => \("([a-z0-9_.]+)", "([^"]+)", Unit::(\w+)\),$/gm)].map((m) => ({ key: m[1], label: m[2], unit: unit(m[3]) }));
  assert.ok(rows.length > 10, "found the metrics! block");
  assert.deepEqual(METRICS, rows);
});

test("a Mac without a part reports null, never zero", () => {
  for (const m of METRICS) assert.equal(metricValue(m.key, empty), null, m.key);
});

test("reads values like the Rust side", () => {
  const s: Snapshot = {
    ...empty,
    thermal_pressure: "heavy",
    cpu: { total_pct: 42, user_pct: 30, system_pct: 12, idle_pct: 58, cores: [], clusters: [], load_average: [1.5, 0, 0], frequency_mhz: null, frequency_is_measured: false },
    memory: { total: 200, used: 50, app: 0, wired: 0, compressed: 0, cached: 0, free: 0, swap_total: 0, swap_used: 7, pressure: "warning", page_ins_per_s: 0, page_outs_per_s: 3 },
    network: { interfaces: [
      { name: "lo0", kind: "loopback", up: true, rx_bps: 9999, tx_bps: 9999, rx_packets_ps: 0, tx_packets_ps: 0, rx_errors: 0, tx_errors: 0, rx_total: 0, tx_total: 0, addresses: [] },
      { name: "en0", kind: "wifi", up: true, rx_bps: 100, tx_bps: 20, rx_packets_ps: 0, tx_packets_ps: 0, rx_errors: 0, tx_errors: 0, rx_total: 0, tx_total: 0, addresses: [] },
    ] },
    sensors: { temperatures: [{ key: "a", label: "a", kind: "cpu", celsius: 61 }, { key: "b", label: "b", kind: "storage", celsius: 70 }], fans: [{ index: 0, label: "f", rpm: 1200, min_rpm: null, max_rpm: null }, { index: 1, label: "g", rpm: 1800, min_rpm: null, max_rpm: null }] },
  };
  assert.equal(metricValue("cpu.total", s), 42);
  assert.equal(metricValue("memory.used_pct", s), 25);
  assert.equal(metricValue("memory.pressure", s), 1);
  assert.equal(metricValue("thermal.pressure", s), 2);
  assert.equal(metricValue("net.rx_bps", s), 100, "loopback is left out");
  assert.equal(metricValue("temp.cpu", s), 61);
  assert.equal(metricValue("temp.max", s), 70);
  assert.equal(metricValue("fan.rpm", s), 1800);
  assert.equal(metricValue("battery.percent", s), null);
});

test("the startup disk is the root volume, not the first one", () => {
  const s: Snapshot = { ...empty, disks: { io: [], volumes: [
    { name: "Data", mount_point: "/Volumes/Data", fs_type: "apfs", total: 100, free: 90 },
    { name: "Mac", mount_point: "/", fs_type: "apfs", total: 200, free: 50 },
  ] } };
  assert.equal(metricValue("disk.free_pct", s), 25);
  assert.equal(metricValue("disk.read_bps", s), null, "no disk statistics is null, not 0");
});
