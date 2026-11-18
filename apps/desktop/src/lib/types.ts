// The shapes the Rust side sends. They mirror `macmetry-core`'s model one to one (snake_case, as serde writes it).

export type Arch = "x86_64" | "arm64" | "unknown";
export type Unit = "percent" | "bytes" | "bytes_per_second" | "celsius" | "rpm" | "watts" | "level" | "per_second";
export type ThermalPressure = "nominal" | "moderate" | "heavy" | "trapping" | "sleeping";
export type CoreKind = "performance" | "efficiency" | "unknown";
export type MemoryPressure = "normal" | "warning" | "critical";
export type InterfaceKind = "wifi" | "ethernet" | "loopback" | "virtual" | "other";
export type SensorKind = "cpu" | "gpu" | "memory" | "storage" | "battery" | "ambient" | "power" | "other";
export type ChargeState = "charging" | "discharging" | "charged" | "not_charging" | "unknown";

export interface SystemInfo {
  model: string;
  chip: string;
  arch: Arch;
  os_version: string;
  os_build: string;
  physical_cores: number;
  logical_cores: number;
  performance_cores: number | null;
  efficiency_cores: number | null;
  memory_bytes: number;
  boot_time_ms: number;
}

export interface CoreUsage { index: number; kind: CoreKind; usage_pct: number; frequency_mhz: number | null }
export interface ClusterUsage { name: string; usage_pct: number; frequency_mhz: number | null }
export interface CpuUsage {
  total_pct: number;
  user_pct: number;
  system_pct: number;
  idle_pct: number;
  cores: CoreUsage[];
  clusters: ClusterUsage[];
  load_average: [number, number, number];
  frequency_mhz: number | null;
  frequency_is_measured: boolean;
}
export interface Memory {
  total: number;
  used: number;
  app: number;
  wired: number;
  compressed: number;
  cached: number;
  free: number;
  swap_total: number;
  swap_used: number;
  pressure: MemoryPressure;
  page_ins_per_s: number;
  page_outs_per_s: number;
}
export interface Volume { name: string; mount_point: string; fs_type: string; total: number; free: number }
export interface DiskIo { name: string; read_bps: number; write_bps: number; read_ops: number; write_ops: number }
export interface DiskStats { volumes: Volume[]; io: DiskIo[] }
export interface NetInterface {
  name: string;
  kind: InterfaceKind;
  up: boolean;
  rx_bps: number;
  tx_bps: number;
  rx_packets_ps: number;
  tx_packets_ps: number;
  rx_errors: number;
  tx_errors: number;
  rx_total: number;
  tx_total: number;
  addresses: string[];
}
export interface Gpu {
  name: string;
  utilization_pct: number | null;
  memory_used: number | null;
  memory_total: number | null;
  core_clock_mhz: number | null;
  memory_clock_mhz: number | null;
  temperature_c: number | null;
  power_w: number | null;
  fan_rpm: number | null;
}
export interface Temperature { key: string; label: string; kind: SensorKind; celsius: number }
export interface Fan { index: number; label: string; rpm: number; min_rpm: number | null; max_rpm: number | null }
export interface Sensors { temperatures: Temperature[]; fans: Fan[] }
export interface PowerDraw { cpu_w: number | null; gpu_w: number | null; ane_w: number | null; dram_w: number | null; system_w: number | null }
export interface Battery {
  percent: number;
  state: ChargeState;
  on_ac_power: boolean;
  health_pct: number | null;
  cycle_count: number | null;
  temperature_c: number | null;
  voltage_v: number | null;
  current_a: number | null;
  power_w: number | null;
  minutes_remaining: number | null;
  adapter_w: number | null;
}
export interface ProcessSample {
  pid: number;
  name: string;
  user: string;
  cpu_pct: number;
  memory_bytes: number;
  threads: number;
  disk_read_bps: number;
  disk_write_bps: number;
}

export interface Snapshot {
  timestamp_ms: number;
  elapsed_ms: number;
  thermal_pressure: ThermalPressure | null;
  cpu: CpuUsage | null;
  memory: Memory | null;
  disks: DiskStats | null;
  network: { interfaces: NetInterface[] } | null;
  gpus: Gpu[];
  sensors: Sensors | null;
  power: PowerDraw | null;
  battery: Battery | null;
  processes: ProcessSample[];
}

export interface Capabilities {
  cpu: boolean;
  memory: boolean;
  disks: boolean;
  network: boolean;
  gpu: boolean;
  temperatures: boolean;
  fans: boolean;
  power: boolean;
  battery: boolean;
  thermal_pressure: boolean;
  processes: boolean;
}

export interface MetricInfo { key: string; label: string; unit: Unit }
export interface AppInfo {
  version: string;
  system: SystemInfo;
  capabilities: Capabilities;
  metrics: MetricInfo[];
  data_dir: string;
}

/// One reading reduced to metric values, in the order of `AppInfo.metrics`.
export interface LivePoint { ts: number; v: (number | null)[] }

export interface Point { ts_ms: number; avg: number; min: number; max: number }
export interface Series { metric: string; resolution_ms: number; points: Point[] }

export interface HistoryStats {
  raw_rows: number;
  minute_rows: number;
  oldest_ms: number | null;
  newest_ms: number | null;
  file_bytes: number;
}

export type Comparator = "above" | "below";
export interface Rule {
  id: string;
  name: string;
  metric: string;
  comparator: Comparator;
  threshold: number;
  for_secs: number;
  cooldown_secs: number;
  enabled: boolean;
}

export interface Settings {
  language: "system" | "en" | "pt-BR";
  theme: "system" | "dark" | "light";
  interval_ms: number;
  record_history: boolean;
  menu_bar_enabled: boolean;
  menu_bar_items: string[];
  hide_dock_icon: boolean;
  keep_running: boolean;
  notifications: boolean;
  alerts: Rule[];
}

export interface AlertEvent {
  rule_id: string;
  name: string;
  kind: "fired" | "resolved";
  message: string;
  value: number;
  at_ms: number;
}
