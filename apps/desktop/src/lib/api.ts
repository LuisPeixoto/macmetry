// The one place the interface talks to the outside. In the app it calls the Rust commands; in a plain browser it uses
// the mock, so the interface can be designed and tested without the app. `?mock=m3` or `?mock=intel` picks the Mac.

import { MockBackend, type Profile } from "./mock.ts";
import type { AlertEvent, AppInfo, HistoryStats, LivePoint, Series, Settings, Snapshot } from "./types.ts";

export interface Backend {
  appInfo(): Promise<AppInfo>;
  latest(): Promise<Snapshot | null>;
  recent(count: number): Promise<LivePoint[]>;
  queryHistory(metrics: string[], fromMs: number, toMs: number, maxPoints: number): Promise<Series[]>;
  historyStats(): Promise<HistoryStats>;
  clearHistory(): Promise<void>;
  /** Asks where to save, then writes the file. The path, or `null` when the person cancelled. */
  exportHistory(metrics: string[], fromMs: number, toMs: number, format: "csv" | "json", includeRange: boolean): Promise<string | null>;
  getSettings(): Promise<Settings>;
  setSettings(settings: Settings): Promise<Settings>;
  alertStatus(): Promise<string[]>;
  showMainWindow(): Promise<void>;
  quitApp(): Promise<void>;
  onSnapshot(callback: (snapshot: Snapshot) => void): Promise<() => void>;
  onAlert(callback: (alert: AlertEvent) => void): Promise<() => void>;
}

export const inApp = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriBackend(): Promise<Backend> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  return {
    appInfo: () => invoke("app_info"),
    latest: () => invoke("latest"),
    recent: (count) => invoke("recent", { count }),
    queryHistory: (metrics, fromMs, toMs, maxPoints) => invoke("query_history", { metrics, fromMs, toMs, maxPoints }),
    historyStats: () => invoke("history_stats"),
    clearHistory: () => invoke("clear_history"),
    exportHistory: (metrics, fromMs, toMs, format, includeRange) => invoke("export_history", { metrics, fromMs, toMs, format, includeRange }),
    getSettings: () => invoke("get_settings"),
    setSettings: (settings) => invoke("set_settings", { settings }),
    alertStatus: () => invoke("alert_status"),
    showMainWindow: () => invoke("show_main_window"),
    quitApp: () => invoke("quit_app"),
    onSnapshot: (cb) => listen<Snapshot>("snapshot", (e) => cb(e.payload)),
    onAlert: (cb) => listen<AlertEvent>("alert", (e) => cb(e.payload)),
  };
}

function mockProfile(): Profile {
  const asked = new URLSearchParams(globalThis.location?.search ?? "").get("mock");
  return asked === "intel" ? "intel" : "m3";
}

let backend: Promise<Backend> | null = null;

/** The backend for this page: the app's commands, or the mock in a browser. Created once. */
export function getBackend(): Promise<Backend> {
  backend ??= inApp ? tauriBackend() : Promise.resolve(new MockBackend(mockProfile()));
  return backend;
}
