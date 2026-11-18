// The metrics the history, the alerts and the charts share. The list and the way each value is read mirror
// `crates/macmetry-core/src/metric.rs`; a test compares the two so they cannot drift apart.

import type { MetricInfo, Snapshot, ThermalPressure, MemoryPressure } from "./types.ts";

export const METRICS: MetricInfo[] = [
  { key: "cpu.total", label: "CPU usage", unit: "percent" },
  { key: "cpu.user", label: "CPU user", unit: "percent" },
  { key: "cpu.system", label: "CPU system", unit: "percent" },
  { key: "cpu.load1", label: "Load average (1 min)", unit: "level" },
  { key: "thermal.pressure", label: "Thermal pressure", unit: "level" },
  { key: "memory.used_pct", label: "Memory used", unit: "percent" },
  { key: "memory.used", label: "Memory used", unit: "bytes" },
  { key: "memory.pressure", label: "Memory pressure", unit: "level" },
  { key: "memory.swap_used", label: "Swap used", unit: "bytes" },
  { key: "memory.page_outs", label: "Page outs", unit: "per_second" },
  { key: "disk.read_bps", label: "Disk read", unit: "bytes_per_second" },
  { key: "disk.write_bps", label: "Disk write", unit: "bytes_per_second" },
  { key: "disk.free_pct", label: "Startup disk free", unit: "percent" },
  { key: "net.rx_bps", label: "Network download", unit: "bytes_per_second" },
  { key: "net.tx_bps", label: "Network upload", unit: "bytes_per_second" },
  { key: "gpu.utilization", label: "GPU usage", unit: "percent" },
  { key: "gpu.temperature", label: "GPU temperature", unit: "celsius" },
  { key: "temp.cpu", label: "CPU temperature", unit: "celsius" },
  { key: "temp.max", label: "Hottest sensor", unit: "celsius" },
  { key: "fan.rpm", label: "Fastest fan", unit: "rpm" },
  { key: "power.cpu", label: "CPU power", unit: "watts" },
  { key: "power.gpu", label: "GPU power", unit: "watts" },
  { key: "power.system", label: "System power", unit: "watts" },
  { key: "battery.percent", label: "Battery charge", unit: "percent" },
  { key: "battery.temperature", label: "Battery temperature", unit: "celsius" },
];

export const THERMAL_LEVEL: Record<ThermalPressure, number> = { nominal: 0, moderate: 1, heavy: 2, trapping: 3, sleeping: 4 };
export const MEMORY_LEVEL: Record<MemoryPressure, number> = { normal: 0, warning: 1, critical: 2 };

const max = (values: number[]): number | null => (values.length ? Math.max(...values) : null);
const nums = (values: (number | null | undefined)[]): number[] => values.filter((v): v is number => typeof v === "number" && Number.isFinite(v));

/** Reads one metric from a snapshot. `null` means the Mac does not provide it, which is not the same as 0. */
export function metricValue(key: string, s: Snapshot): number | null {
  const mem = s.memory;
  switch (key) {
    case "cpu.total": return s.cpu?.total_pct ?? null;
    case "cpu.user": return s.cpu?.user_pct ?? null;
    case "cpu.system": return s.cpu?.system_pct ?? null;
    case "cpu.load1": return s.cpu?.load_average[0] ?? null;
    case "thermal.pressure": return s.thermal_pressure ? THERMAL_LEVEL[s.thermal_pressure] : null;
    case "memory.used_pct": return mem && mem.total > 0 ? (mem.used / mem.total) * 100 : null;
    case "memory.used": return mem?.used ?? null;
    case "memory.pressure": return mem ? MEMORY_LEVEL[mem.pressure] : null;
    case "memory.swap_used": return mem?.swap_used ?? null;
    case "memory.page_outs": return mem?.page_outs_per_s ?? null;
    case "disk.read_bps": return s.disks && s.disks.io.length ? s.disks.io.reduce((a, d) => a + d.read_bps, 0) : null;
    case "disk.write_bps": return s.disks && s.disks.io.length ? s.disks.io.reduce((a, d) => a + d.write_bps, 0) : null;
    case "disk.free_pct": {
      const v = s.disks?.volumes.find((x) => x.mount_point === "/") ?? s.disks?.volumes[0];
      return v && v.total > 0 ? (v.free / v.total) * 100 : null;
    }
    // Loopback traffic is the Mac talking to itself, so it is left out of the totals.
    case "net.rx_bps": return s.network ? s.network.interfaces.filter((i) => i.kind !== "loopback").reduce((a, i) => a + i.rx_bps, 0) : null;
    case "net.tx_bps": return s.network ? s.network.interfaces.filter((i) => i.kind !== "loopback").reduce((a, i) => a + i.tx_bps, 0) : null;
    case "gpu.utilization": return max(nums(s.gpus.map((g) => g.utilization_pct)));
    case "gpu.temperature": return max(nums(s.gpus.map((g) => g.temperature_c)));
    case "temp.cpu": return max(nums((s.sensors?.temperatures ?? []).filter((t) => t.kind === "cpu").map((t) => t.celsius)));
    case "temp.max": return max(nums((s.sensors?.temperatures ?? []).map((t) => t.celsius)));
    case "fan.rpm": return max(nums((s.sensors?.fans ?? []).map((f) => f.rpm)));
    case "power.cpu": return s.power?.cpu_w ?? null;
    case "power.gpu": return s.power?.gpu_w ?? null;
    case "power.system": return s.power?.system_w ?? null;
    case "battery.percent": return s.battery?.percent ?? null;
    case "battery.temperature": return s.battery?.temperature_c ?? null;
    default: return null;
  }
}

export const metricInfo = (key: string): MetricInfo | undefined => METRICS.find((m) => m.key === key);
