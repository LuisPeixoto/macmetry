<script lang="ts">
  import { onMount } from "svelte";
  import { live } from "./lib/stores/live.svelte.ts";

  onMount(() => {
    live.start();
    return () => live.dispose();
  });
</script>

<main>
  {#if live.info && live.snapshot}
    <h1>macmetry</h1>
    <p>{live.info.system.model} · {live.info.system.chip}</p>
    <p>CPU {live.snapshot.cpu?.total_pct.toFixed(1)}% · {live.series["cpu.total"]?.length ?? 0} points</p>
  {:else}
    <p>Starting…</p>
  {/if}
</main>
