<script lang="ts">
  import { live } from "../../stores/live.svelte.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { t } from "../../stores/i18n.svelte.ts";
  import { getBackend } from "../../api.ts";
  import { METRICS } from "../../metrics.ts";
  import { assignColors, MAX_SERIES } from "../../colors.ts";
  import { align, groupByUnit, MAX_POINTS, RANGES, refreshEvery, type Range } from "../../history.ts";
  import type { HistoryStats, Series } from "../../types.ts";
  import Card from "../ui/Card.svelte";
  import Chip from "../ui/Chip.svelte";
  import Button from "../ui/Button.svelte";
  import Icon from "../ui/Icon.svelte";
  import EmptyState from "../ui/EmptyState.svelte";
  import Segmented from "../ui/Segmented.svelte";
  import LineChart from "../charts/LineChart.svelte";
  import ViewHeader from "../layout/ViewHeader.svelte";

  let range = $state<Range>("1h");
  let selected = $state<string[]>(["cpu.total", "memory.used_pct"]);
  let data = $state.raw<Series[]>([]);
  let stats = $state.raw<HistoryStats | null>(null);
  let error = $state("");
  let notice = $state("");
  let loaded = $state(false);

  // A metric is offered when this Mac has produced a value for it; asking for a battery chart on a desktop is pointless.
  const available = $derived(METRICS.filter((m) => (live.series[m.key] ?? []).some((v) => v !== null)));
  const rangeMs = $derived(RANGES[range]);

  async function load() {
    try {
      const backend = await getBackend();
      const keys = selected.filter((k) => available.some((m) => m.key === k));
      const now = Date.now();
      data = keys.length ? await backend.queryHistory(keys, now - rangeMs, now, MAX_POINTS) : [];
      stats = await backend.historyStats();
      error = "";
    } catch (e) {
      error = String(e);
    } finally {
      loaded = true;
    }
  }

  // Look again when the range or the metrics change, and every so often while the page is open.
  $effect(() => {
    range; selected; available.length;
    load();
    const timer = setInterval(load, refreshEvery(rangeMs));
    return () => clearInterval(timer);
  });

  function toggle(key: string) {
    selected = selected.includes(key) ? selected.filter((k) => k !== key) : selected.length < MAX_SERIES ? [...selected, key] : selected;
  }

  const metricOf = (key: string) => METRICS.find((m) => m.key === key)!;
  const colors = $derived(assignColors(selected));
  const charts = $derived(
    groupByUnit(data.map((s) => ({ ...metricOf(s.metric), series: s }))).map((g) => {
      const a = align(g.metrics.map((m) => m.series));
      return {
        unit: g.unit,
        times: a.times,
        resolutionMs: a.resolutionMs,
        series: g.metrics.map((m, i) => ({
          key: m.key,
          label: t(`metric.${m.key}`),
          color: colors[m.key],
          values: a.avg[i],
          // With one line on the chart, the lowest and highest value of each point show as a band behind it.
          band: g.metrics.length === 1 ? { min: a.min[i], max: a.max[i] } : undefined,
        })),
      };
    }),
  );

  async function exportAs(format: "csv" | "json") {
    try {
      const now = Date.now();
      const path = await (await getBackend()).exportHistory(selected, now - rangeMs, now, format, true);
      notice = path ? t("history.exported", { path }) : "";
      error = "";
    } catch (e) {
      error = String(e);
    }
  }
  const spanText = $derived(stats?.oldest_ms && stats.newest_ms ? t("history.covers", { from: fmt.dateTime(stats.oldest_ms), to: fmt.dateTime(stats.newest_ms) }) : t("history.empty_short"));
  const empty = $derived(loaded && !error && (stats?.raw_rows ?? 0) + (stats?.minute_rows ?? 0) === 0);
</script>

<ViewHeader title={t("nav.history")} subtitle={spanText}>
  {#snippet actions()}
    <Segmented label={t("history.range")} value={range} onchange={(v) => (range = v)} options={(Object.keys(RANGES) as Range[]).map((r) => ({ value: r, label: t(`history.range.${r}`) }))} />
    <Button onclick={() => exportAs("csv")} disabled={!selected.length || empty}><Icon name="download" size={14} />CSV</Button>
    <Button onclick={() => exportAs("json")} disabled={!selected.length || empty}><Icon name="download" size={14} />JSON</Button>
  {/snippet}
</ViewHeader>

<div class="scroll">
  <div class="stack">
    {#if error}<p class="msg bad">{error}</p>{/if}
    {#if notice}<p class="msg good">{notice}</p>{/if}

    <Card title={t("history.metrics")} icon="history">
      <div class="chips">
        {#each available as m (m.key)}
          <Chip label={t(`metric.${m.key}`)} on={selected.includes(m.key)} color={colors[m.key]} disabled={!selected.includes(m.key) && selected.length >= MAX_SERIES} onclick={() => toggle(m.key)} />
        {/each}
      </div>
      <p class="muted hint">{t("history.hint")}</p>
    </Card>

    {#if empty}
      <Card><EmptyState icon="history" title={t("history.empty")} text={t("history.empty_text")} /></Card>
    {:else if !selected.length}
      <Card><EmptyState icon="chart" title={t("history.pick")} /></Card>
    {:else}
      {#each charts as c (c.unit)}
        <Card title={t(`history.unit.${c.unit}`)} icon="chart">
          <LineChart times={c.times} series={c.series} unit={c.unit} height={240} resolutionMs={c.resolutionMs} fill={c.series.length === 1} label={t(`history.unit.${c.unit}`)} />
        </Card>
      {/each}
    {/if}
  </div>
</div>

<style>
  .chips { display: flex; flex-wrap: wrap; gap: 8px; }
  .hint { margin: 12px 0 0; font-size: 12px; }
  .msg { margin: 0; padding: 10px 14px; border-radius: 10px; background: var(--card); border: 1px solid var(--line); font-size: 12.5px; user-select: text; -webkit-user-select: text; }
  .msg.bad { color: var(--bad); border-color: color-mix(in srgb, var(--bad) 40%, transparent); }
  .msg.good { color: var(--good); border-color: color-mix(in srgb, var(--good) 40%, transparent); }
</style>
