// Alert rules as the person edits them. The rule stores a number in the metric's own unit (percent, degrees, bytes);
// the form shows a friendlier one (megabytes, named pressure levels). These functions convert and describe, and are tested.

import type { Comparator, MetricInfo, Rule, Unit } from "./types.ts";

/** How a threshold is shown and typed for a unit. `factor` turns what is typed into the stored value. */
export interface Field { kind: "number" | "level"; factor: number; suffix: string; step: number }

export function fieldFor(metric: MetricInfo): Field {
  const unit: Unit = metric.unit;
  switch (unit) {
    case "percent": return { kind: "number", factor: 1, suffix: "%", step: 1 };
    case "celsius": return { kind: "number", factor: 1, suffix: "°C", step: 1 };
    case "rpm": return { kind: "number", factor: 1, suffix: "rpm", step: 50 };
    case "watts": return { kind: "number", factor: 1, suffix: "W", step: 1 };
    case "bytes": return { kind: "number", factor: 1024 ** 3, suffix: "GB", step: 0.5 };
    case "bytes_per_second": return { kind: "number", factor: 1_000_000, suffix: "MB/s", step: 1 };
    case "per_second": return { kind: "number", factor: 1, suffix: "/s", step: 10 };
    case "level": return metric.key.endsWith("pressure") ? { kind: "level", factor: 1, suffix: "", step: 1 } : { kind: "number", factor: 1, suffix: "", step: 0.5 };
  }
}

/** What goes in the box for a stored threshold. */
export const toInput = (stored: number, field: Field): number => Math.round((stored / field.factor) * 1000) / 1000;
/** The stored value for what was typed. */
export const fromInput = (typed: number, field: Field): number => typed * field.factor;

/**
 * A pressure alert means "when it reaches level X". The rule stores a threshold half a step below X, so that "above"
 * catches X and anything higher (level 2, critical, is stored as 1.5).
 */
export const levelToThreshold = (level: number): number => level - 0.5;
export const thresholdToLevel = (threshold: number): number => Math.ceil(threshold);

/** A new rule for a metric, with a threshold that is a sensible starting point and not zero. */
export function newRule(metric: MetricInfo, existing: readonly Rule[]): Rule {
  const taken = new Set(existing.map((r) => r.id));
  let n = 1;
  while (taken.has(`custom_${n}`)) n++;
  const start: Record<string, [Comparator, number]> = {
    percent: ["above", 90], celsius: ["above", 85], rpm: ["above", 4000], watts: ["above", 40], level: ["above", 1.5],
    bytes: ["above", 2 * 1024 ** 3], bytes_per_second: ["above", 50_000_000], per_second: ["above", 100],
  };
  const [comparator, threshold] = start[metric.unit] ?? ["above", 1];
  return { id: `custom_${n}`, name: metric.label, metric: metric.key, comparator, threshold, for_secs: 60, cooldown_secs: 900, enabled: true };
}

/** A problem with a rule that the form can tell the person about before the app does, or `null`. */
export function problem(rule: Rule): string | null {
  if (!rule.name.trim()) return "name";
  if (!Number.isFinite(rule.threshold)) return "threshold";
  return null;
}

/** The durations offered for "for how long" and "stay quiet after", in seconds. */
export const FOR_OPTIONS = [0, 30, 60, 120, 300, 600, 1800] as const;
export const COOLDOWN_OPTIONS = [60, 300, 900, 1800, 3600, 21600, 86400] as const;

export function durationLabel(seconds: number): string {
  if (seconds === 0) return "0 s";
  if (seconds < 60) return `${seconds} s`;
  if (seconds < 3600) return `${Math.round(seconds / 60)} min`;
  return `${Math.round(seconds / 3600)} h`;
}

/**
 * The name to show. A suggested rule keeps the English name the Rust side saved, so it is shown in the language in use
 * while the name is still the original; once the person renames it, their name is shown as it is.
 */
export function displayName(rule: Rule, original: readonly Rule[], translated: (id: string) => string): string {
  const base = original.find((r) => r.id === rule.id);
  return base && base.name === rule.name ? translated(rule.id) : rule.name;
}
