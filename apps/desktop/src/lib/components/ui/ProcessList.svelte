<script lang="ts">
  // A short list of processes with a bar for the column that matters. The full, sortable table is the Processes view.
  import type { ProcessSample } from "../../types.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { C } from "../../colors.ts";

  let { processes, by = "cpu", limit = 6 }: { processes: ProcessSample[]; by?: "cpu" | "memory"; limit?: number } = $props();
  const rows = $derived(
    [...processes].sort((a, b) => (by === "cpu" ? b.cpu_pct - a.cpu_pct : b.memory_bytes - a.memory_bytes)).slice(0, limit),
  );
  const top = $derived(Math.max(1, ...rows.map((p) => (by === "cpu" ? p.cpu_pct : p.memory_bytes))));
</script>

<ul>
  {#each rows as p (p.pid)}
    {@const v = by === "cpu" ? p.cpu_pct : p.memory_bytes}
    <li>
      <span class="name" title={p.name}>{p.name}</span>
      <span class="meter"><i style="width:{(v / top) * 100}%;background:{by === 'cpu' ? C.cpu : C.memory}"></i></span>
      <b class="num">{by === "cpu" ? `${fmt.num(p.cpu_pct, 1)}%` : fmt.bytes(p.memory_bytes)}</b>
    </li>
  {/each}
</ul>

<style>
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 9px; }
  li { display: grid; grid-template-columns: minmax(110px, 1.3fr) minmax(60px, 2fr) 76px; align-items: center; gap: 14px; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  b { text-align: right; font-weight: 600; }
</style>
