<script lang="ts">
  // One headline number with its label. The value wears the text colour; `tone` only adds a small status mark.
  let { label, value, hint, tone }: { label: string; value: string; hint?: string; tone?: "good" | "warn" | "bad" } = $props();
</script>

<div class="tile">
  <div class="eyebrow">{label}</div>
  <div class="mid">
    {#if tone}<span class="dot {tone}" aria-hidden="true"></span>{/if}{value}
  </div>
  {#if hint}<div class="muted hint">{hint}</div>{/if}
</div>

<style>
  .tile { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .hint { font-size: 12px; overflow-wrap: anywhere; line-height: 1.35; }
  .dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 8px; vertical-align: middle; }
  .good { background: var(--good); } .warn { background: var(--warn); } .bad { background: var(--bad); }
</style>
