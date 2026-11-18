<script lang="ts">
  // A small area chart with no axes, for a card. The path is computed in a fixed box and stretched to the width.
  import { areaPath, extent, linePath, type Value } from "../../chart.ts";

  let { values, color = "var(--s1)", height = 38, max, min = 0 }: { values: Value[]; color?: string; height?: number; max?: number; min?: number } = $props();
  const id = `sp${Math.random().toString(36).slice(2, 8)}`;
  const W = 100;

  const geometry = $derived.by(() => {
    const e = extent([values]);
    const hi = max ?? (e ? Math.max(e[1] * 1.15, min + 1e-9) : 1);
    const lo = min;
    const x = (i: number) => (values.length <= 1 ? 0 : (i / (values.length - 1)) * W);
    const y = (v: number) => height - 2 - ((Math.min(Math.max(v, lo), hi) - lo) / (hi - lo || 1)) * (height - 4);
    return { line: linePath(values, x, y), area: areaPath(values, x, y, height) };
  });
</script>

<svg viewBox="0 0 {W} {height}" preserveAspectRatio="none" width="100%" {height} aria-hidden="true">
  <defs>
    <linearGradient id={id} x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color={color} stop-opacity="0.32" />
      <stop offset="1" stop-color={color} stop-opacity="0" />
    </linearGradient>
  </defs>
  <path d={geometry.area} fill="url(#{id})" />
  <path d={geometry.line} fill="none" stroke={color} stroke-width="1.6" stroke-linejoin="round" stroke-linecap="round" vector-effect="non-scaling-stroke" />
</svg>
