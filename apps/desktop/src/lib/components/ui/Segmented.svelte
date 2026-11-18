<script lang="ts" generics="T extends string">
  let { options, value, onchange, label }: { options: { value: T; label: string }[]; value: T; onchange: (v: T) => void; label?: string } = $props();
</script>

<div class="seg" role="radiogroup" aria-label={label}>
  {#each options as o (o.value)}
    <button type="button" role="radio" aria-checked={o.value === value} class:on={o.value === value} onclick={() => onchange(o.value)}>{o.label}</button>
  {/each}
</div>

<style>
  .seg { display: inline-flex; padding: 2px; gap: 2px; background: var(--card-2); border: 1px solid var(--line); border-radius: 10px; }
  button { border: 0; background: transparent; color: var(--text-2); padding: 4px 11px; border-radius: 8px; font-size: 12px; font-weight: 550; cursor: pointer; transition: background 0.12s, color 0.12s; }
  button:hover { color: var(--text); }
  button.on { background: var(--card); color: var(--text); box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25), 0 0 0 1px var(--line-2); }
</style>
