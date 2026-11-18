<script lang="ts">
  // One vertical bar per core, so an unevenly loaded CPU shows at a glance. Hovering a bar names the core.
  import type { CoreUsage } from "../../types.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { loadLevel } from "../../chart.ts";

  let { cores, color = "var(--s1)", height = 96 }: { cores: CoreUsage[]; color?: string; height?: number } = $props();
</script>

<div class="bars" style="height:{height}px" role="img" aria-label={cores.map((c) => `${c.index + 1}: ${fmt.percent(c.usage_pct)}`).join(", ")}>
  {#each cores as c (c.index)}
    <div class="col" title="#{c.index + 1} · {fmt.percent(c.usage_pct)}{c.frequency_mhz ? ` · ${fmt.freq(c.frequency_mhz)}` : ''}">
      <div class="track"><i style="height:{Math.max(2, c.usage_pct)}%;background:{loadLevel(c.usage_pct) === 'hot' ? 'var(--bad)' : loadLevel(c.usage_pct) === 'busy' ? 'var(--warn)' : color}"></i></div>
      <span class="num">{c.index + 1}</span>
    </div>
  {/each}
</div>

<style>
  .bars { display: flex; gap: 6px; align-items: stretch; }
  .col { flex: 1; min-width: 14px; max-width: 72px; display: flex; flex-direction: column; gap: 4px; }
  .track { flex: 1; background: var(--grid); border-radius: 5px; display: flex; align-items: flex-end; overflow: hidden; }
  i { width: 100%; border-radius: 5px; transition: height 0.4s; }
  span { font-size: 10px; color: var(--text-3); text-align: center; }
</style>
