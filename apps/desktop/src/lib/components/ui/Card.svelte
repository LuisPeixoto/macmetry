<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  let { title, icon, children, actions, pad = true }: { title?: string; icon?: string; children: Snippet; actions?: Snippet; pad?: boolean } = $props();
</script>

<section class="card" class:flush={!pad}>
  {#if title || actions}
    <header>
      <div class="title">
        {#if icon}<span class="ico"><Icon name={icon as never} size={16} /></span>{/if}
        {#if title}<h3>{title}</h3>{/if}
      </div>
      {#if actions}<div class="actions">{@render actions()}</div>{/if}
    </header>
  {/if}
  {@render children()}
</section>

<style>
  .flush { padding: 0; overflow: hidden; }
  header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; gap: 12px; }
  .flush header { padding: 14px 18px 0; }
  .title { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .ico { color: var(--text-3); display: inline-flex; }
  h3 { margin: 0; font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--text-3); white-space: nowrap; }
  .actions { display: flex; align-items: center; gap: 8px; }
</style>
