<script lang="ts">
  let { checked, onchange, label }: { checked: boolean; onchange: (v: boolean) => void; label: string } = $props();
</script>

<button type="button" role="switch" aria-checked={checked} aria-label={label} class:on={checked} onclick={() => onchange(!checked)}><span></span></button>

<style>
  button { width: 38px; height: 22px; border-radius: 999px; border: 1px solid var(--line-2); background: var(--card-2); padding: 2px; cursor: pointer; transition: background 0.15s; flex: none; }
  span { display: block; width: 16px; height: 16px; border-radius: 50%; background: var(--text-2); transition: transform 0.15s, background 0.15s; }
  button.on { background: var(--accent); border-color: transparent; }
  button.on span { transform: translateX(16px); background: #fff; }
</style>
