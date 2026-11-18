<script lang="ts">
  // The main time chart: thin lines, recessive grid, a crosshair with a tooltip, a legend when there are two or more
  // series, and a table view for anyone who prefers numbers. Missing values break the line; they are never drawn as 0.
  import { areaPath, extent, lastValue, linePath, nearestIndex, niceTicks, yRange, type Value } from "../../chart.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { t } from "../../stores/i18n.svelte.ts";
  import type { Unit } from "../../types.ts";
  import Icon from "../ui/Icon.svelte";

  export interface ChartSeries { key: string; label: string; color: string; values: Value[] }
  let {
    times, series, unit, height = 220, yMax, fill = true, legend = true, table = true, resolutionMs = 1000, label = "",
  }: {
    times: number[]; series: ChartSeries[]; unit: Unit; height?: number; yMax?: number; fill?: boolean; legend?: boolean; table?: boolean;
    /** How much time one point covers; decides whether the axis shows seconds and whether a tooltip shows the date. */
    resolutionMs?: number; label?: string;
  } = $props();

  const M = { l: 56, r: 12, t: 10, b: 24 };
  let width = $state(600);
  let hover = $state<number | null>(null);
  let showTable = $state(false);
  const gid = `lc${Math.random().toString(36).slice(2, 8)}`;

  const n = $derived(times.length);
  const innerW = $derived(Math.max(10, width - M.l - M.r));
  const innerH = $derived(height - M.t - M.b);

  const scale = $derived.by(() => {
    const e = extent(series.map((s) => s.values));
    const [lo, hi] = yRange(e, unit === "percent" ? "percent" : "other", { max: yMax });
    const ticks = unit === "percent" ? [0, 25, 50, 75, 100] : niceTicks(lo, hi, 4);
    const top = Math.max(hi, ticks.at(-1) ?? hi);
    return { lo, hi: top, ticks };
  });
  const x = $derived((i: number) => M.l + (n <= 1 ? 0 : (i / (n - 1)) * innerW));
  const y = $derived((v: number) => M.t + innerH - ((Math.min(Math.max(v, scale.lo), scale.hi) - scale.lo) / (scale.hi - scale.lo || 1)) * innerH);

  const paths = $derived(series.map((s) => ({ s, line: linePath(s.values, x, y), area: fill ? areaPath(s.values, x, y, M.t + innerH) : "" })));
  const showSeconds = $derived(resolutionMs < 60_000);
  const longRange = $derived(n > 1 && times[n - 1] - times[0] > 20 * 3600_000);
  const xTicks = $derived.by(() => {
    if (n < 2) return [];
    const count = Math.max(2, Math.min(6, Math.floor(innerW / 110)));
    return Array.from({ length: count }, (_, k) => Math.round((k / (count - 1)) * (n - 1)));
  });
  const stamp = (ms: number) => (longRange ? fmt.dateTime(ms) : fmt.clock(ms, showSeconds));
  const fmtValue = (v: Value) => fmt.value(unit, v);

  function move(e: PointerEvent) {
    const box = (e.currentTarget as SVGElement).getBoundingClientRect();
    hover = n ? nearestIndex(e.clientX - box.left, M.l, innerW, n) : null;
  }
  const tip = $derived(hover === null ? null : { i: hover, left: Math.min(Math.max(x(hover) + 14, 8), width - 190) });
</script>

<div class="wrap" bind:clientWidth={width}>
  {#if (legend && series.length > 1) || table}
    <div class="top">
      {#if legend && series.length > 1}
        <ul class="legend">
          {#each series as s (s.key)}
            <li><i style="background:{s.color}"></i><span class="sub">{s.label}</span><b class="num">{fmtValue(lastValue(s.values))}</b></li>
          {/each}
        </ul>
      {:else}<span></span>{/if}
      {#if table}
        <button type="button" class="toggle" onclick={() => (showTable = !showTable)} aria-pressed={showTable} title={showTable ? t("chart.show_chart") : t("chart.show_table")}>
          <Icon name={showTable ? "chart" : "table"} size={14} />
        </button>
      {/if}
    </div>
  {/if}

  {#if showTable}
    <div class="tablewrap" style="max-height:{height}px">
      <table>
        <thead><tr><th>{t("chart.time")}</th>{#each series as s (s.key)}<th>{s.label}</th>{/each}</tr></thead>
        <tbody>
          {#each times.map((_, k) => n - 1 - k).slice(0, 60) as i (i)}
            <tr><td class="num">{stamp(times[i])}</td>{#each series as s (s.key)}<td class="num">{fmtValue(s.values[i] ?? null)}</td>{/each}</tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <svg {width} {height} role="img" aria-label={label || series.map((s) => `${s.label} ${fmtValue(lastValue(s.values))}`).join(", ")}
      onpointermove={move} onpointerleave={() => (hover = null)}>
      <defs>
        {#each series as s (s.key)}
          <linearGradient id="{gid}-{s.key}" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stop-color={s.color} stop-opacity="0.26" /><stop offset="1" stop-color={s.color} stop-opacity="0" />
          </linearGradient>
        {/each}
      </defs>
      {#each scale.ticks as tick (tick)}
        <line x1={M.l} x2={M.l + innerW} y1={y(tick)} y2={y(tick)} stroke="var(--grid)" />
        <text x={M.l - 8} y={y(tick) + 3.5} text-anchor="end" class="axis">{fmtValue(tick)}</text>
      {/each}
      {#each xTicks as i (i)}
        <text x={x(i)} y={height - 6} text-anchor={i === 0 ? "start" : i === n - 1 ? "end" : "middle"} class="axis">{stamp(times[i])}</text>
      {/each}
      {#each paths as p, k (p.s.key)}
        {#if k === 0 && fill}<path d={p.area} fill="url(#{gid}-{p.s.key})" />{/if}
        <path d={p.line} fill="none" stroke={p.s.color} stroke-width="2" stroke-linejoin="round" stroke-linecap="round" />
      {/each}
      {#if tip}
        <line x1={x(tip.i)} x2={x(tip.i)} y1={M.t} y2={M.t + innerH} stroke="var(--line-2)" stroke-dasharray="3 3" />
        {#each series as s (s.key)}
          {@const v = s.values[tip.i]}
          {#if typeof v === "number"}<circle cx={x(tip.i)} cy={y(v)} r="4.5" fill={s.color} stroke="var(--card)" stroke-width="2" />{/if}
        {/each}
      {/if}
    </svg>
    {#if tip}
      <div class="tip" style="left:{tip.left}px;top:{M.t + 4}px">
        <div class="muted">{longRange || resolutionMs >= 60_000 ? fmt.dateTime(times[tip.i]) : fmt.clock(times[tip.i], true)}</div>
        {#each series as s (s.key)}
          <div class="trow"><i style="background:{s.color}"></i><span class="sub">{s.label}</span><b class="num">{fmtValue(s.values[tip.i] ?? null)}</b></div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .wrap { position: relative; width: 100%; }
  .top { display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px; min-height: 22px; }
  .legend { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 4px 18px; }
  li { display: flex; align-items: center; gap: 7px; font-size: 12px; }
  i { width: 10px; height: 10px; border-radius: 3px; flex: none; }
  b { font-weight: 600; }
  svg { display: block; overflow: visible; }
  .axis { font-size: 10.5px; fill: var(--text-3); font-variant-numeric: tabular-nums; }
  .toggle { border: 1px solid var(--line); background: var(--card-2); color: var(--text-2); width: 26px; height: 24px; border-radius: 8px; display: grid; place-items: center; cursor: pointer; }
  .toggle:hover, .toggle[aria-pressed="true"] { color: var(--text); border-color: var(--line-2); }
  .tip { position: absolute; z-index: 5; pointer-events: none; min-width: 150px; padding: 8px 10px; border-radius: 10px; background: var(--card-2); border: 1px solid var(--line-2); box-shadow: 0 10px 28px -8px rgba(0, 0, 0, 0.5); font-size: 12px; }
  .trow { display: flex; align-items: center; gap: 7px; margin-top: 4px; }
  .trow b { margin-left: auto; padding-left: 14px; }
  .tablewrap { overflow: auto; border: 1px solid var(--line); border-radius: 10px; }
  table { width: 100%; border-collapse: collapse; font-size: 12px; }
  th { position: sticky; top: 0; background: var(--card-2); text-align: left; font-weight: 600; color: var(--text-2); padding: 6px 10px; }
  td { padding: 5px 10px; border-top: 1px solid var(--line); }
</style>
