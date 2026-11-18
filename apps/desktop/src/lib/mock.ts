// A stand-in for the Rust side, used when the page runs in a plain browser (design work, screenshots, tests). It makes
// believable readings for two kinds of Mac, so every card can be seen with and without the hardware it needs.

import { METRICS, metricValue } from "./metrics.ts";
import type {
  AlertEvent, AppInfo, Capabilities, ChargeState, HistoryStats, InterfaceKind, LivePoint, ProcessSample, Series, Settings,
  Snapshot, SystemInfo,
} from "./types.ts";
import { defaultSettings } from "./defaults.ts";

export type Profile = "intel" | "m3";

const GB = 1024 ** 3;
const MB = 1024 ** 2;

const PROFILES: Record<Profile, { system: SystemInfo; capabilities: Capabilities }> = {
  intel: {
    system: {
      model: "MacPro7,1", chip: "Intel Xeon W-3235 @ 3.30GHz", arch: "x86_64", os_version: "26.2", os_build: "25C56",
      physical_cores: 12, logical_cores: 24, performance_cores: null, efficiency_cores: null, memory_bytes: 96 * GB,
      boot_time_ms: Date.now() - 9 * 86400_000,
    },
    capabilities: { cpu: true, memory: true, disks: true, network: true, gpu: true, temperatures: true, fans: true, power: true, battery: false, thermal_pressure: true, processes: true },
  },
  m3: {
    system: {
      model: "Mac15,6", chip: "Apple M3 Pro", arch: "arm64", os_version: "15.2", os_build: "24C101",
      physical_cores: 11, logical_cores: 11, performance_cores: 5, efficiency_cores: 6, memory_bytes: 18 * GB,
      boot_time_ms: Date.now() - 3 * 86400_000,
    },
    capabilities: { cpu: true, memory: true, disks: true, network: true, gpu: true, temperatures: true, fans: true, power: true, battery: true, thermal_pressure: true, processes: true },
  },
};

/** A repeatable pseudo-random number in [0, 1) for a time and a salt, so a chart redrawn later looks the same. */
function noise(t: number, salt: number): number {
  let x = (Math.floor(t / 1000) * 374761393 + salt * 668265263) | 0;
  x = Math.imul(x ^ (x >>> 13), 1274126177);
  return ((x ^ (x >>> 16)) >>> 0) / 4294967296;
}
const wave = (t: number, period: number, phase = 0) => Math.sin((t / 1000 / period) * Math.PI * 2 + phase);
const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

const APPS = [
  ["Xcode", 14, 3200], ["Google Chrome", 9, 1400], ["Google Chrome Helper (Renderer)", 7, 880], ["Slack", 4, 640],
  ["WindowServer", 6, 410], ["Code Helper (Renderer)", 5, 760], ["node", 11, 520], ["Spotify", 3, 380], ["Figma", 4, 920],
  ["Finder", 1, 190], ["kernel_task", 3, 1900], ["Terminal", 2, 150], ["mds_stores", 2, 300], ["Docker Desktop", 5, 2100],
  ["Mail", 1, 410], ["Safari", 3, 980], ["Notes", 0.5, 160], ["backupd", 0.6, 80], ["coreaudiod", 0.8, 40], ["Music", 1, 330],
] as const;

export function snapshotAt(profile: Profile, t: number): Snapshot {
  const { system, capabilities: caps } = PROFILES[profile];
  const m3 = profile === "m3";
  const load = clamp(26 + 18 * wave(t, 40) + 10 * wave(t, 9, 1) + 14 * noise(t, 1), 2, 98);
  const nCores = system.logical_cores;
  const cores = Array.from({ length: nCores }, (_, i) => {
    const kind = m3 ? (i < 6 ? "efficiency" : "performance") : "unknown";
    const bias = m3 && kind === "efficiency" ? -0.45 : 0.1;
    return { index: i, kind: kind as "efficiency" | "performance" | "unknown", usage_pct: clamp(load * (1 + bias) + (noise(t, 20 + i) - 0.5) * 55, 0, 100), frequency_mhz: m3 ? (kind === "efficiency" ? 1100 + 800 * noise(t, 40 + i) : 1500 + 2700 * (load / 100)) : null };
  });
  const memUsed = system.memory_bytes * clamp(0.58 + 0.04 * wave(t, 120) + 0.01 * noise(t, 3), 0, 1);
  const hot = 49 + load * 0.26 + 2.5 * wave(t, 25);
  const charge = 72 - ((t / 1000) % 3000) / 100;
  const power = m3 ? 2.2 + load * 0.2 + 1.5 * noise(t, 5) : 28 + load * 1.1 + 6 * noise(t, 5);
  const iface = (name: string, kind: InterfaceKind, rx: number, tx: number) => ({
    name, kind, up: true, rx_bps: rx, tx_bps: tx, rx_packets_ps: rx / 900, tx_packets_ps: tx / 1100, rx_errors: 0, tx_errors: 0,
    rx_total: 41 * GB + t * 1500, tx_total: 6 * GB + t * 300, addresses: [],
  });
  const burst = Math.max(0, wave(t, 31) - 0.2) * 6.5 * MB * (0.4 + noise(t, 8));
  const procs: ProcessSample[] = APPS.map(([name, cpu, memMb], i) => ({
    pid: 300 + i * 37, name, user: name === "WindowServer" || name === "kernel_task" ? "_windowserver" : "luis",
    cpu_pct: Math.max(0, cpu * (0.5 + noise(t, 60 + i) + 0.35 * wave(t, 17 + i))), memory_bytes: memMb * MB * (0.97 + 0.06 * noise(t, 90 + i)),
    threads: 4 + (i * 7) % 40, disk_read_bps: noise(t, 120 + i) > 0.8 ? 2 * MB * noise(t, 140 + i) : 0, disk_write_bps: noise(t, 160 + i) > 0.85 ? MB * noise(t, 180 + i) : 0,
  })).sort((a, b) => b.cpu_pct - a.cpu_pct);

  const state: ChargeState = "discharging";
  return {
    timestamp_ms: t, elapsed_ms: 1000, thermal_pressure: hot > 82 ? "moderate" : "nominal",
    cpu: {
      total_pct: load, user_pct: load * 0.68, system_pct: load * 0.32, idle_pct: 100 - load, cores,
      clusters: m3 ? [{ name: "Efficiency", usage_pct: load * 0.55, frequency_mhz: 1400 }, { name: "Performance", usage_pct: load * 1.1, frequency_mhz: 2400 + 1600 * (load / 100) }] : [],
      load_average: [load / 10, load / 11, load / 12], frequency_mhz: m3 ? 1900 + 1500 * (load / 100) : 3300, frequency_is_measured: m3,
    },
    memory: {
      total: system.memory_bytes, used: memUsed, app: memUsed * 0.55, wired: memUsed * 0.2, compressed: memUsed * 0.25, cached: system.memory_bytes * 0.18,
      free: system.memory_bytes * 0.08, swap_total: 4 * GB, swap_used: m3 ? 1.3 * GB : 0.2 * GB, pressure: "normal", page_ins_per_s: 12 * noise(t, 12), page_outs_per_s: 3 * noise(t, 13),
    },
    disks: {
      volumes: [
        { name: m3 ? "Macintosh HD" : "MACOS", mount_point: "/", fs_type: "apfs", total: 994 * GB, free: 412 * GB },
        ...(m3 ? [] : [{ name: "SSD512", mount_point: "/Volumes/SSD512", fs_type: "apfs", total: 512 * GB, free: 58 * GB }]),
      ],
      io: [{ name: "disk0", read_bps: 40 * MB * Math.max(0, wave(t, 27)) * noise(t, 14), write_bps: 18 * MB * Math.max(0, wave(t, 19, 2)) * noise(t, 15), read_ops: 300 * noise(t, 16), write_ops: 120 * noise(t, 17) }],
    },
    network: { interfaces: [iface(m3 ? "en0" : "en0", m3 ? "wifi" : "ethernet", 0.3 * MB + burst, 0.08 * MB + burst * 0.15), iface("lo0", "loopback", 4000, 4000), iface("utun3", "virtual", 0, 0)] },
    gpus: [m3
      ? { name: "Apple M3 Pro (14 cores)", utilization_pct: clamp(load * 0.5 + 8 * noise(t, 18), 0, 100), memory_used: 2.1 * GB, memory_total: null, core_clock_mhz: 700 + 600 * (load / 100), memory_clock_mhz: null, temperature_c: null, power_w: 0.8 + 3 * (load / 100), fan_rpm: null }
      : { name: "AMD Radeon RX 6600 XT", utilization_pct: clamp(load * 0.3 + 5 * noise(t, 18), 0, 100), memory_used: 2.9 * GB, memory_total: 8 * GB, core_clock_mhz: 111 + 1900 * (load / 100) ** 2, memory_clock_mhz: 1990, temperature_c: 48 + load * 0.3, power_w: 14 + load * 0.5, fan_rpm: 683 }],
    sensors: caps.temperatures ? {
      temperatures: m3
        ? [
          { key: "Tp01", label: "CPU performance core 1", kind: "cpu", celsius: hot + 3 }, { key: "Tp05", label: "CPU performance core 2", kind: "cpu", celsius: hot + 1 },
          { key: "Tp09", label: "CPU performance core 3", kind: "cpu", celsius: hot }, { key: "Te05", label: "CPU efficiency core 1", kind: "cpu", celsius: hot - 6 },
          { key: "Tg0f", label: "GPU cluster 1", kind: "gpu", celsius: hot - 3 }, { key: "NAND CH0 temp", label: "NAND CH0 temp", kind: "storage", celsius: 34 + 4 * wave(t, 90) },
          { key: "gas gauge battery", label: "gas gauge battery", kind: "battery", celsius: 31 + wave(t, 200) },
        ]
        : [
          { key: "TC0P", label: "CPU proximity", kind: "cpu", celsius: hot }, { key: "TC0D", label: "CPU die", kind: "cpu", celsius: hot + 2 },
          ...Array.from({ length: 12 }, (_, i) => ({ key: `TC${i}C`, label: `CPU core ${i + 1}`, kind: "cpu" as const, celsius: hot - 3 + 5 * noise(t, 200 + i) })),
        ],
      fans: m3 ? [{ index: 0, label: "Fan 1", rpm: 1180 + load * 22, min_rpm: 1200, max_rpm: 5779 }, { index: 1, label: "Fan 2", rpm: 1210 + load * 22, min_rpm: 1200, max_rpm: 6241 }]
        : [{ index: 0, label: "Fan 1", rpm: 1100 + load * 14, min_rpm: null, max_rpm: 3315 }, { index: 1, label: "Fan 2", rpm: 0, min_rpm: null, max_rpm: 3315 }, { index: 2, label: "Fan 3", rpm: 0, min_rpm: null, max_rpm: 3315 }, { index: 3, label: "Fan 4", rpm: 0, min_rpm: null, max_rpm: 3315 }, { index: 4, label: "Fan 5", rpm: 1000 + load * 11, min_rpm: null, max_rpm: 3315 }],
    } : null,
    power: m3
      ? { cpu_w: power, gpu_w: 0.8 + 3 * (load / 100), ane_w: 0.05 * noise(t, 22), dram_w: 0.9 + 0.8 * noise(t, 23), system_w: power + 5.5 }
      : { cpu_w: power, gpu_w: null, ane_w: null, dram_w: null, system_w: null },
    battery: caps.battery ? { percent: charge, state, on_ac_power: false, health_pct: 94, cycle_count: 187, temperature_c: 31 + wave(t, 200), voltage_v: 12.1, current_a: -(power + 5.5) / 12.1, power_w: -(power + 5.5), minutes_remaining: Math.round((charge / 100) * 60 * 9), adapter_w: null } : null,
    processes: procs,
  };
}

export class MockBackend {
  private settings: Settings = defaultSettings();
  private listeners = new Set<(s: Snapshot) => void>();
  private alertListeners = new Set<(a: AlertEvent) => void>();
  private timer: ReturnType<typeof setInterval> | null = null;
  constructor(readonly profile: Profile) {}

  private start() {
    if (this.timer) return;
    this.timer = setInterval(() => {
      const s = snapshotAt(this.profile, Date.now());
      this.listeners.forEach((l) => l(s));
    }, this.settings.interval_ms);
  }

  async appInfo(): Promise<AppInfo> {
    const p = PROFILES[this.profile];
    return { version: "0.1.0", system: p.system, capabilities: p.capabilities, metrics: METRICS, data_dir: "~/Library/Application Support/macmetry" };
  }
  async latest() { return snapshotAt(this.profile, Date.now()); }
  async recent(count: number): Promise<LivePoint[]> {
    const now = Date.now();
    return Array.from({ length: Math.min(count, 600) }, (_, i) => {
      const ts = now - (Math.min(count, 600) - i) * 1000;
      const s = snapshotAt(this.profile, ts);
      return { ts, v: METRICS.map((m) => metricValue(m.key, s)) };
    });
  }
  async queryHistory(metrics: string[], fromMs: number, toMs: number, maxPoints: number): Promise<Series[]> {
    const step = Math.max(1000, Math.ceil((toMs - fromMs) / Math.max(2, maxPoints) / 1000) * 1000);
    return metrics.map((metric) => {
      const points = [];
      for (let ts = fromMs - (fromMs % step); ts <= toMs; ts += step) {
        const probe = (k: number) => metricValue(metric, snapshotAt(this.profile, ts + k * (step / 4)));
        const samples = [0, 1, 2, 3].map(probe).filter((v): v is number => v !== null);
        if (!samples.length) continue;
        points.push({ ts_ms: ts, avg: samples.reduce((a, b) => a + b, 0) / samples.length, min: Math.min(...samples), max: Math.max(...samples) });
      }
      return { metric, resolution_ms: step, points };
    });
  }
  async historyStats(): Promise<HistoryStats> {
    const now = Date.now();
    return { raw_rows: 86_400, minute_rows: 43_200, oldest_ms: now - 30 * 86400_000, newest_ms: now, file_bytes: 38 * MB };
  }
  async clearHistory() {}
  async exportHistory(): Promise<string | null> { return "~/Downloads/macmetry-history.csv"; }
  async getSettings() { return this.settings; }
  async setSettings(s: Settings) { this.settings = s; return s; }
  async alertStatus() { return [] as string[]; }
  async showMainWindow() {}
  async quitApp() {}
  async onSnapshot(cb: (s: Snapshot) => void) { this.listeners.add(cb); this.start(); return () => { this.listeners.delete(cb); }; }
  async onAlert(cb: (a: AlertEvent) => void) { this.alertListeners.add(cb); return () => { this.alertListeners.delete(cb); }; }
}
