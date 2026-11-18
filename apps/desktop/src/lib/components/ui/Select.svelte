<script lang="ts" generics="T extends string | number">
  let { value, options, onchange, label, width }: { value: T; options: { value: T; label: string }[]; onchange: (v: T) => void; label: string; width?: string } = $props();
</script>

<select aria-label={label} style={width ? `width:${width}` : undefined} value={String(value)} onchange={(e) => {
  const raw = (e.currentTarget as HTMLSelectElement).value;
  onchange((typeof value === "number" ? Number(raw) : raw) as T);
}}>
  {#each options as o (o.value)}<option value={String(o.value)}>{o.label}</option>{/each}
</select>

<style>
  select { appearance: none; -webkit-appearance: none; height: 30px; padding: 0 28px 0 11px; border-radius: 9px; border: 1px solid var(--line-2); background: var(--card-2) url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6' fill='none' stroke='%237c879d' stroke-width='1.6' stroke-linecap='round'%3E%3Cpath d='M1 1l4 4 4-4'/%3E%3C/svg%3E") no-repeat right 10px center; color: var(--text); font: inherit; cursor: pointer; max-width: 100%; }
  select:hover { border-color: var(--text-3); }
</style>
