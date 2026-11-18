// The live readings: the latest snapshot, a short history of every metric for the sparklines, and the processes.

import { getBackend } from "../api.ts";
import { METRICS, metricValue } from "../metrics.ts";
import type { AlertEvent, AppInfo, ProcessSample, Snapshot } from "../types.ts";

/** How many points each live chart holds. At one reading a second that is five minutes. */
export const LIVE_POINTS = 300;

class LiveStore {
  info = $state.raw<AppInfo | null>(null);
  snapshot = $state.raw<Snapshot | null>(null);
  /** The busiest processes. Not every reading carries them, so the last list stays until a new one arrives. */
  processes = $state.raw<ProcessSample[]>([]);
  /** Timestamps of the points below, oldest first. */
  times = $state.raw<number[]>([]);
  /** The last `LIVE_POINTS` values of each metric by key; `null` where the Mac did not provide it. */
  series = $state.raw<Record<string, (number | null)[]>>({});
  alerts = $state.raw<AlertEvent[]>([]);
  ready = $state(false);
  private stop: (() => void)[] = [];

  async start() {
    if (this.ready) return;
    const backend = await getBackend();
    this.info = await backend.appInfo();
    const recent = await backend.recent(LIVE_POINTS);
    const series: Record<string, (number | null)[]> = {};
    METRICS.forEach((m, i) => (series[m.key] = recent.map((p) => p.v[i] ?? null)));
    this.series = series;
    this.times = recent.map((p) => p.ts);
    const first = await backend.latest();
    if (first) this.accept(first);
    this.ready = true;
    this.stop.push(await backend.onSnapshot((s) => this.accept(s)));
    this.stop.push(await backend.onAlert((a) => (this.alerts = [a, ...this.alerts].slice(0, 50))));
  }

  private accept(snapshot: Snapshot) {
    this.snapshot = snapshot;
    if (snapshot.processes.length) this.processes = snapshot.processes;
    // New objects on every update, so anything that watches them redraws.
    const series: Record<string, (number | null)[]> = {};
    for (const m of METRICS) series[m.key] = [...(this.series[m.key] ?? []), metricValue(m.key, snapshot)].slice(-LIVE_POINTS);
    this.series = series;
    this.times = [...this.times, snapshot.timestamp_ms].slice(-LIVE_POINTS);
  }

  dispose() {
    this.stop.forEach((f) => f());
    this.stop = [];
    this.ready = false;
  }
}

export const live = new LiveStore();
