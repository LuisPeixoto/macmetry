//! The data model. Every field that a Mac may not provide is an `Option` (or an empty list), so an Intel desktop
//! without a battery and an Apple silicon laptop without a discrete GPU use the same types.
//!
//! All structs use `#[serde(default)]`, so a snapshot stored by an older version still loads after fields are added.

use serde::{Deserialize, Serialize};

/// The CPU architecture of the Mac.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Arch {
    /// Intel (x86_64).
    X86_64,
    /// Apple silicon (arm64).
    Arm64,
    #[default]
    Unknown,
}

/// Facts about the Mac that do not change while it runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SystemInfo {
    /// Hardware model identifier, for example `MacPro7,1`.
    pub model: String,
    /// Marketing name of the chip, for example `Apple M3 Pro` or the Intel brand string.
    pub chip: String,
    pub arch: Arch,
    pub os_version: String,
    pub os_build: String,
    pub physical_cores: u32,
    pub logical_cores: u32,
    /// Performance cores (Apple silicon only).
    pub performance_cores: Option<u32>,
    /// Efficiency cores (Apple silicon only).
    pub efficiency_cores: Option<u32>,
    pub memory_bytes: u64,
    /// When the Mac booted, in milliseconds since the Unix epoch.
    pub boot_time_ms: u64,
}

/// How hard the system is working to stay cool, as macOS reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThermalPressure {
    Nominal,
    Moderate,
    Heavy,
    Trapping,
    Sleeping,
}

impl ThermalPressure {
    /// A number that grows with the pressure, for charts and alert thresholds.
    pub fn level(self) -> u8 {
        match self {
            Self::Nominal => 0,
            Self::Moderate => 1,
            Self::Heavy => 2,
            Self::Trapping => 3,
            Self::Sleeping => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CoreKind {
    Performance,
    Efficiency,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CoreUsage {
    pub index: u32,
    pub kind: CoreKind,
    pub usage_pct: f64,
    /// Average frequency over the interval (Apple silicon only).
    pub frequency_mhz: Option<f64>,
}

/// A group of cores that share a clock, such as the performance cluster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ClusterUsage {
    pub name: String,
    pub usage_pct: f64,
    pub frequency_mhz: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CpuUsage {
    pub total_pct: f64,
    pub user_pct: f64,
    pub system_pct: f64,
    pub idle_pct: f64,
    pub cores: Vec<CoreUsage>,
    pub clusters: Vec<ClusterUsage>,
    /// Load averages over 1, 5 and 15 minutes.
    pub load_average: [f64; 3],
    /// Average frequency across the active cores; the nominal frequency on Intel.
    pub frequency_mhz: Option<f64>,
    /// Whether `frequency_mhz` is measured (Apple silicon) or only the nominal value (Intel).
    pub frequency_is_measured: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryPressure {
    #[default]
    Normal,
    Warning,
    Critical,
}

impl MemoryPressure {
    pub fn level(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::Warning => 1,
            Self::Critical => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Memory {
    pub total: u64,
    /// App memory + wired + compressed, the figure Activity Monitor calls "Memory Used".
    pub used: u64,
    pub app: u64,
    pub wired: u64,
    pub compressed: u64,
    /// File cache that can be released when needed.
    pub cached: u64,
    pub free: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub pressure: MemoryPressure,
    pub page_ins_per_s: f64,
    pub page_outs_per_s: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Volume {
    pub name: String,
    pub mount_point: String,
    pub fs_type: String,
    pub total: u64,
    pub free: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DiskIo {
    /// BSD name, for example `disk0`.
    pub name: String,
    pub read_bps: f64,
    pub write_bps: f64,
    pub read_ops: f64,
    pub write_ops: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DiskStats {
    pub volumes: Vec<Volume>,
    pub io: Vec<DiskIo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceKind {
    Wifi,
    Ethernet,
    Loopback,
    Virtual,
    #[default]
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct NetInterface {
    pub name: String,
    pub kind: InterfaceKind,
    pub up: bool,
    pub rx_bps: f64,
    pub tx_bps: f64,
    pub rx_packets_ps: f64,
    pub tx_packets_ps: f64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_total: u64,
    pub tx_total: u64,
    /// Addresses assigned to the interface on this Mac.
    pub addresses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct NetworkStats {
    pub interfaces: Vec<NetInterface>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Gpu {
    pub name: String,
    /// Busiest of the engines the driver reports, in percent.
    pub utilization_pct: Option<f64>,
    pub memory_used: Option<u64>,
    pub memory_total: Option<u64>,
    pub core_clock_mhz: Option<f64>,
    pub memory_clock_mhz: Option<f64>,
    pub temperature_c: Option<f64>,
    pub power_w: Option<f64>,
    pub fan_rpm: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SensorKind {
    Cpu,
    Gpu,
    Memory,
    Storage,
    Battery,
    Ambient,
    Power,
    #[default]
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Temperature {
    /// The raw sensor key, for example `TC0P`. Kept so people can look it up.
    pub key: String,
    pub label: String,
    pub kind: SensorKind,
    pub celsius: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Fan {
    pub index: u32,
    pub label: String,
    pub rpm: f64,
    pub min_rpm: Option<f64>,
    pub max_rpm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Sensors {
    pub temperatures: Vec<Temperature>,
    pub fans: Vec<Fan>,
}

impl Sensors {
    /// The hottest sensor, which is what a single "temperature" number should show.
    pub fn hottest(&self) -> Option<&Temperature> {
        self.temperatures.iter().max_by(|a, b| a.celsius.total_cmp(&b.celsius))
    }

    /// The hottest sensor of one kind.
    pub fn hottest_of(&self, kind: SensorKind) -> Option<&Temperature> {
        self.temperatures.iter().filter(|t| t.kind == kind).max_by(|a, b| a.celsius.total_cmp(&b.celsius))
    }
}

/// Power draw by component, in watts. Apple silicon reports the components; Intel reports what the SMC knows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PowerDraw {
    pub cpu_w: Option<f64>,
    pub gpu_w: Option<f64>,
    pub ane_w: Option<f64>,
    pub dram_w: Option<f64>,
    pub system_w: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChargeState {
    Charging,
    Discharging,
    /// Full and on the charger.
    Charged,
    /// On the charger but holding the charge, as with optimized charging or a limit.
    NotCharging,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Battery {
    pub percent: f64,
    pub state: ChargeState,
    pub on_ac_power: bool,
    /// Maximum capacity as a percentage of the design capacity.
    pub health_pct: Option<f64>,
    pub cycle_count: Option<u32>,
    pub temperature_c: Option<f64>,
    pub voltage_v: Option<f64>,
    /// Positive while charging, negative while discharging.
    pub current_a: Option<f64>,
    pub power_w: Option<f64>,
    pub minutes_remaining: Option<u32>,
    /// Rated power of the connected charger.
    pub adapter_w: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProcessSample {
    pub pid: i32,
    pub name: String,
    pub user: String,
    /// Percent of one core, so a process using two cores shows 200.
    pub cpu_pct: f64,
    /// Physical memory footprint in bytes.
    pub memory_bytes: u64,
    pub threads: u32,
    pub disk_read_bps: f64,
    pub disk_write_bps: f64,
}

/// One reading of everything. Parts the Mac cannot provide are `None` or empty.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Snapshot {
    /// Milliseconds since the Unix epoch.
    pub timestamp_ms: u64,
    /// Time since the previous sample; 0 on the first one (rates are 0 then).
    pub elapsed_ms: u32,
    pub thermal_pressure: Option<ThermalPressure>,
    pub cpu: Option<CpuUsage>,
    pub memory: Option<Memory>,
    pub disks: Option<DiskStats>,
    pub network: Option<NetworkStats>,
    pub gpus: Vec<Gpu>,
    pub sensors: Option<Sensors>,
    pub power: Option<PowerDraw>,
    pub battery: Option<Battery>,
    pub processes: Vec<ProcessSample>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_object_is_a_valid_snapshot() {
        let s: Snapshot = serde_json::from_str("{}").unwrap();
        assert_eq!(s, Snapshot::default());
    }

    #[test]
    fn older_and_newer_json_still_loads() {
        // A field this version does not know (added later) is ignored; a field it expects but the file lacks
        // (added since) takes its default.
        let json = r#"{"timestamp_ms": 7, "cpu": {"total_pct": 12.5, "from_the_future": true}}"#;
        let s: Snapshot = serde_json::from_str(json).unwrap();
        assert_eq!(s.timestamp_ms, 7);
        assert_eq!(s.cpu.unwrap().total_pct, 12.5);
        assert!(s.memory.is_none());
    }

    #[test]
    fn a_snapshot_survives_a_round_trip() {
        let s = Snapshot {
            thermal_pressure: Some(ThermalPressure::Moderate),
            gpus: vec![Gpu { name: "AMD Radeon".into(), temperature_c: Some(69.0), ..Default::default() }],
            ..Default::default()
        };
        let back: Snapshot = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn thermal_pressure_is_ordered() {
        assert!(ThermalPressure::Nominal < ThermalPressure::Heavy);
        assert_eq!(ThermalPressure::Trapping.level(), 3);
    }
}
