<script lang="ts">
  import type { Snippet } from "svelte";
  let { label, description, children }: { label: string; description?: string; children: Snippet } = $props();
</script>

<div class="row">
  <div class="text">
    <div class="label">{label}</div>
    {#if description}<div class="muted desc">{description}</div>{/if}
  </div>
  <div class="control">{@render children()}</div>
</div>

<style>
  .row { display: flex; align-items: center; justify-content: space-between; gap: 24px; padding: 12px 0; border-top: 1px solid var(--line); }
  .row:first-child { border-top: 0; padding-top: 2px; }
  .label { font-weight: 560; }
  .desc { font-size: 12px; margin-top: 2px; max-width: 56ch; }
  .control { display: flex; align-items: center; gap: 8px; flex: none; flex-wrap: wrap; justify-content: flex-end; }
</style>
