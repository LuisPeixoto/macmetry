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
