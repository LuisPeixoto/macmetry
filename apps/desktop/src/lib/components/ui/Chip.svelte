<script lang="ts">
  let { label, on, onclick, color, disabled = false }: { label: string; on: boolean; onclick: () => void; color?: string; disabled?: boolean } = $props();
</script>

<button type="button" class:on aria-pressed={on} {disabled} {onclick}>
  {#if on && color}<i style="background:{color}"></i>{/if}{label}
</button>

<style>
  button { display: inline-flex; align-items: center; gap: 7px; height: 28px; padding: 0 12px; border-radius: 999px; border: 1px solid var(--line-2); background: transparent; color: var(--text-2); font-size: 12.5px; font-weight: 520; cursor: pointer; transition: background 0.1s, color 0.1s, border-color 0.1s; }
  button:hover:not(:disabled) { color: var(--text); border-color: var(--text-3); }
  button.on { background: color-mix(in srgb, var(--accent) 18%, transparent); color: var(--text); border-color: color-mix(in srgb, var(--accent) 55%, transparent); }
  button:disabled { opacity: 0.4; cursor: not-allowed; }
  i { width: 9px; height: 9px; border-radius: 3px; }
</style>
