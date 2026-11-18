import type { Settings } from "./types.ts";

/** The settings a new install starts with. Mirrors `Settings::default()` in the Rust shell. */
export function defaultSettings(): Settings {
  return {
    language: "system",
    theme: "system",
    interval_ms: 1000,
    record_history: true,
    menu_bar_enabled: true,
    menu_bar_items: ["cpu", "temp"],
    hide_dock_icon: false,
    keep_running: true,
    notifications: true,
    alerts: [
      { id: "disk_low", name: "Startup disk almost full", metric: "disk.free_pct", comparator: "below", threshold: 10, for_secs: 60, cooldown_secs: 21600, enabled: true },
      { id: "temp_critical", name: "Very hot sensor", metric: "temp.max", comparator: "above", threshold: 95, for_secs: 30, cooldown_secs: 900, enabled: true },
      { id: "cpu_high", name: "CPU busy for a while", metric: "cpu.total", comparator: "above", threshold: 90, for_secs: 120, cooldown_secs: 900, enabled: false },
      { id: "memory_critical", name: "Memory pressure is critical", metric: "memory.pressure", comparator: "above", threshold: 1.5, for_secs: 30, cooldown_secs: 900, enabled: false },
      { id: "thermal_heavy", name: "The Mac is slowing down to cool off", metric: "thermal.pressure", comparator: "above", threshold: 1.5, for_secs: 30, cooldown_secs: 900, enabled: false },
      { id: "battery_low", name: "Battery is low", metric: "battery.percent", comparator: "below", threshold: 15, for_secs: 0, cooldown_secs: 1800, enabled: false },
    ],
  };
}
