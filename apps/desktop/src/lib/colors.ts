// What each measured thing looks like. A colour follows the entity, not its rank: CPU is always blue, memory always
// violet, wherever they appear. The slots are the validated categorical order (see docs/design.md).

export const C = {
  cpu: "var(--s1)",
  cpuSystem: "var(--s2)",
  memory: "var(--s7)",
  gpu: "var(--s3)",
  diskRead: "var(--s4)",
  diskWrite: "var(--s5)",
  netDown: "var(--s2)",
  netUp: "var(--s6)",
  temperature: "var(--s8)",
  power: "var(--s5)",
  battery: "var(--s3)",
  fan: "var(--s1)",
} as const;

/** The status colour of a load or a pressure level: calm, busy, hot. Always paired with a word or an icon. */
export const tone = (level: "ok" | "busy" | "hot"): "good" | "warn" | "bad" => (level === "hot" ? "bad" : level === "busy" ? "warn" : "good");

/** The colour each metric prefers. A colour belongs to the thing measured, so CPU stays blue and memory stays violet. */
const PREFERRED: Record<string, number> = {
  "cpu.total": 1, "cpu.user": 3, "cpu.system": 2, "cpu.load1": 1, "thermal.pressure": 8,
  "memory.used_pct": 7, "memory.used": 7, "memory.pressure": 7, "memory.swap_used": 4, "memory.page_outs": 2,
  "disk.read_bps": 4, "disk.write_bps": 5, "disk.free_pct": 6, "net.rx_bps": 2, "net.tx_bps": 6,
  "gpu.utilization": 3, "gpu.temperature": 5, "temp.cpu": 2, "temp.max": 8, "fan.rpm": 1,
  "power.cpu": 5, "power.gpu": 3, "power.system": 4, "battery.percent": 3, "battery.temperature": 4,
};

/** The most series one chart can carry: the eight slots of the validated palette. */
export const MAX_SERIES = 8;

/**
 * Colours for the metrics on one chart. Each gets its preferred colour; if two want the same one, the later takes
 * the first free slot, so two lines on a chart are never the same colour. The result does not depend on the order the
 * person ticked them in, so adding or removing a metric does not repaint the others.
 */
export function assignColors(keys: readonly string[]): Record<string, string> {
  const order = [...keys].sort((a, b) => a.localeCompare(b));
  const used = new Set<number>();
  const result: Record<string, string> = {};
  for (const key of order) {
    let slot = PREFERRED[key] ?? 1;
    if (used.has(slot)) slot = Array.from({ length: MAX_SERIES }, (_, i) => i + 1).find((s) => !used.has(s)) ?? slot;
    used.add(slot);
    result[key] = `var(--s${slot})`;
  }
  return result;
}
