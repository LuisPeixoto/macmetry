<script lang="ts">
  // One bar split into parts, with a 2px gap between them, and a legend that names each part and its size. The legend
  // text stays in the text colour; the swatch carries the identity.
  export interface Segment { label: string; value: number; color: string; text: string }
  let { segments, total }: { segments: Segment[]; total?: number } = $props();
  const sum = $derived(total ?? segments.reduce((a, s) => a + s.value, 0));
</script>

<div class="bar" role="img" aria-label={segments.map((s) => `${s.label} ${s.text}`).join(", ")}>
  {#each segments as s (s.label)}
    <span style="flex:{Math.max(s.value, 0)};background:{s.color}"></span>
  {/each}
  {#if sum > segments.reduce((a, s) => a + s.value, 0)}<span class="rest" style="flex:{sum - segments.reduce((a, s) => a + s.value, 0)}"></span>{/if}
</div>
<ul class="legend">
  {#each segments as s (s.label)}
    <li><i style="background:{s.color}"></i><span class="sub">{s.label}</span><b class="num">{s.text}</b></li>
  {/each}
</ul>

<style>
  .bar { display: flex; gap: 2px; height: 14px; }
  .bar span { border-radius: 4px; min-width: 3px; }
  .bar .rest { background: var(--grid); }
  .legend { list-style: none; margin: 12px 0 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 6px 18px; }
  li { display: flex; align-items: center; gap: 8px; }
  i { width: 10px; height: 10px; border-radius: 3px; flex: none; }
  b { margin-left: auto; font-weight: 600; }
</style>
