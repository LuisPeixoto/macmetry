<script lang="ts">
  import type { Snippet } from "svelte";
  let { title, subtitle, actions }: { title: string; subtitle?: string; actions?: Snippet } = $props();
</script>

<header data-tauri-drag-region>
  <div class="text" data-tauri-drag-region>
    <h1 data-tauri-drag-region>{title}</h1>
    {#if subtitle}<p data-tauri-drag-region class="sub">{subtitle}</p>{/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
  header { display: flex; align-items: flex-end; justify-content: space-between; gap: 16px; padding: 18px 28px 16px; flex: none; }
  h1 { margin: 0; font-size: 24px; font-weight: 650; letter-spacing: -0.025em; }
  p { margin: 2px 0 0; }
  .actions { display: flex; align-items: center; gap: 10px; }
</style>
