<script lang="ts">
  import type { Snippet } from "svelte";
  let { children, onclick, kind = "normal", disabled = false, title }: { children: Snippet; onclick: () => void; kind?: "normal" | "primary" | "danger"; disabled?: boolean; title?: string } = $props();
</script>

<button type="button" class={kind} {disabled} {title} {onclick}>{@render children()}</button>

<style>
  button { display: inline-flex; align-items: center; gap: 7px; height: 30px; padding: 0 13px; border-radius: 9px; border: 1px solid var(--line-2); background: var(--card-2); color: var(--text); font: inherit; font-weight: 550; cursor: pointer; white-space: nowrap; }
  button:hover:not(:disabled) { border-color: var(--text-3); }
  button:disabled { opacity: 0.45; cursor: not-allowed; }
  .primary { background: var(--grad); color: #fff; border: 0; }
  .primary:hover:not(:disabled) { filter: brightness(1.08); }
  .danger { color: var(--bad); }
  .danger:hover:not(:disabled) { border-color: var(--bad); }
</style>
