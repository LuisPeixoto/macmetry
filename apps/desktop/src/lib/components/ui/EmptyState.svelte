<script lang="ts">
  import Icon from "./Icon.svelte";
  let { icon, title, text }: { icon: string; title: string; text?: string } = $props();
</script>

<div class="empty">
  <div class="glyph"><Icon name={icon as never} size={26} /></div>
  <div class="mid">{title}</div>
  {#if text}<p class="sub">{text}</p>{/if}
</div>

<style>
  .empty { display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; gap: 8px; padding: 56px 24px; }
  .glyph { width: 54px; height: 54px; border-radius: 16px; display: grid; place-items: center; background: var(--card-2); color: var(--text-3); border: 1px solid var(--line); }
  p { max-width: 360px; margin: 0; }
</style>
