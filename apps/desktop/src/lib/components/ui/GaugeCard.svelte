<script lang="ts">
  // The overview's building block: a ring, a headline, a one-line hint and a sparkline. Clicking it opens the view.
  import Gauge from "../charts/Gauge.svelte";
  import Spark from "../charts/Spark.svelte";
  import Icon from "./Icon.svelte";
  import type { Value } from "../../chart.ts";

  let { title, icon, value, big, hint, hint2, spark, color = "var(--s1)", onopen, warn = true }: {
    title: string; icon: string; value: number | null; big: string; hint?: string; hint2?: string; spark: Value[]; color?: string; onopen?: () => void; warn?: boolean;
  } = $props();
</script>

<button type="button" class="card tile" onclick={onopen}>
  <div class="head"><Icon name={icon as never} size={15} /><span class="eyebrow">{title}</span><Icon name="chevron" size={13} /></div>
  <div class="body">
    <Gauge {value} size={96} stroke={10} warnAt={warn}><div class="mid">{big}</div></Gauge>
    <div class="side">
      {#if hint}<div class="sub hint">{hint}</div>{/if}
      {#if hint2}<div class="muted hint">{hint2}</div>{/if}
    </div>
  </div>
  <div class="spark"><Spark values={spark} {color} max={warn ? 100 : undefined} /></div>
</button>

<style>
  .tile { text-align: left; cursor: pointer; display: flex; flex-direction: column; gap: 10px; transition: border-color 0.15s, transform 0.15s; width: 100%; }
  .tile:hover { border-color: var(--line-2); transform: translateY(-1px); }
  .head { display: flex; align-items: center; gap: 7px; color: var(--text-3); }
  .head .eyebrow { flex: 1; }
  .body { display: flex; align-items: center; gap: 12px; min-height: 96px; }
  .side { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .hint { font-size: 12px; overflow-wrap: anywhere; line-height: 1.35; }
  .spark { margin: 0 -4px -6px; }
</style>
