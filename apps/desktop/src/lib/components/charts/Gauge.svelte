<script lang="ts">
  // A 270 degree ring. The ring fills with the value; its colour turns amber and red as the load climbs, the same
  // thresholds everywhere, so a colour always means the same thing.
  import type { Snippet } from "svelte";
  import { loadLevel } from "../../chart.ts";

  let { value, size = 132, stroke = 12, children, warnAt = true, level: forced }: {
    value: number | null; size?: number; stroke?: number; children?: Snippet; warnAt?: boolean;
    /** Sets the colour directly, for a reading where low is the bad end (a battery). */
    level?: "ok" | "busy" | "hot";
  } = $props();
  const id = `g${Math.random().toString(36).slice(2, 8)}`;
  const r = $derived((size - stroke) / 2);
  const c = $derived(2 * Math.PI * r);
  const arc = $derived(0.75 * c);
  const filled = $derived(((value === null ? 0 : Math.min(100, Math.max(0, value))) / 100) * arc);
  const level = $derived(forced ?? (warnAt ? loadLevel(value) : "ok"));
</script>

<div class="gauge" style="width:{size}px;height:{size}px">
  <svg width={size} height={size} viewBox="0 0 {size} {size}" aria-hidden="true">
    <defs>
      <linearGradient id={id} x1="0" y1="1" x2="1" y2="0">
        <stop offset="0" stop-color="#7c6cff" /><stop offset="0.55" stop-color="#3aa0ff" /><stop offset="1" stop-color="#2ee6c8" />
      </linearGradient>
    </defs>
    <g transform="rotate(135 {size / 2} {size / 2})">
      <circle cx={size / 2} cy={size / 2} {r} fill="none" stroke="var(--grid)" stroke-width={stroke} stroke-linecap="round" stroke-dasharray="{arc} {c}" />
      <circle class="value" cx={size / 2} cy={size / 2} {r} fill="none" stroke={level === "hot" ? "var(--bad)" : level === "busy" ? "var(--warn)" : `url(#${id})`}
        stroke-width={stroke} stroke-linecap="round" stroke-dasharray="{Math.max(filled, 0.001)} {c}" />
    </g>
  </svg>
  <div class="center">{#if children}{@render children()}{/if}</div>
</div>

<style>
  .gauge { position: relative; flex: none; }
  .center { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; }
</style>
