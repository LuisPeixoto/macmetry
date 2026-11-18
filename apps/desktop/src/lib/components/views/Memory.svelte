<script lang="ts">
  import { live } from "../../stores/live.svelte.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { t } from "../../stores/i18n.svelte.ts";
  import { C } from "../../colors.ts";
  import Card from "../ui/Card.svelte";
  import StatTile from "../ui/StatTile.svelte";
  import ProcessList from "../ui/ProcessList.svelte";
  import LineChart from "../charts/LineChart.svelte";
  import StackedBar from "../charts/StackedBar.svelte";
  import ViewHeader from "../layout/ViewHeader.svelte";

  const m = $derived(live.snapshot?.memory);
  const series = $derived(live.series);
  const pct = $derived(m && m.total ? (m.used / m.total) * 100 : 0);
  const pressureTone = $derived(m?.pressure === "normal" ? "good" : m?.pressure === "warning" ? "warn" : "bad");
  // The three parts of "used" and the two that macOS gives back when needed, in the order Activity Monitor lists them.
  const segments = $derived(
    m ? [
      { label: t("mem.app"), value: m.app, color: "var(--s7)", text: fmt.bytes(m.app) },
      { label: t("mem.wired"), value: m.wired, color: "var(--s1)", text: fmt.bytes(m.wired) },
      { label: t("mem.compressed"), value: m.compressed, color: "var(--s2)", text: fmt.bytes(m.compressed) },
      { label: t("mem.cached"), value: m.cached, color: "var(--s3)", text: fmt.bytes(m.cached) },
    ] : [],
  );
</script>

<ViewHeader title={t("nav.memory")} subtitle={t("mem.subtitle", { total: fmt.bytes(m?.total ?? 0) })} />

<div class="scroll">
  {#if m}
    <div class="stack">
      <Card>
        <div class="tiles">
          <StatTile label={t("mem.used")} value={fmt.bytes(m.used)} hint={t("mem.of_total", { pct: fmt.percent(pct, 0), total: fmt.bytes(m.total) })} />
          <StatTile label={t("mem.pressure")} value={t(`pressure.${m.pressure}`)} tone={pressureTone} hint={t(`mem.hint.${m.pressure}`)} />
          <StatTile label={t("mem.swap")} value={fmt.bytes(m.swap_used)} hint={m.swap_total ? t("mem.swap_of", { total: fmt.bytes(m.swap_total) }) : undefined} />
          <StatTile label={t("mem.paging")} value="{fmt.num(m.page_outs_per_s, 0)}/s" hint="{t('mem.page_ins')} {fmt.num(m.page_ins_per_s, 0)}/s" />
        </div>
      </Card>

      <Card title={t("mem.breakdown")} icon="memory">
        <StackedBar {segments} total={m.total} />
        <p class="muted note">{t("mem.note")}</p>
      </Card>

      <Card title={t("mem.history")} icon="memory">
        <LineChart times={live.times} height={210} unit="percent" legend={false} label={t("mem.history")}
          series={[{ key: "mem", label: t("metric.memory.used_pct"), color: C.memory, values: series["memory.used_pct"] ?? [] }]} />
      </Card>

      {#if m.swap_total > 0}
        <Card title={t("mem.swap_history")} icon="disk">
          <LineChart times={live.times} height={150} unit="bytes" legend={false} table={false} label={t("mem.swap_history")}
            series={[{ key: "swap", label: t("mem.swap"), color: C.memory, values: series["memory.swap_used"] ?? [] }]} />
        </Card>
      {/if}

      <Card title={t("mem.busiest")} icon="processes">
        <ProcessList processes={live.processes} by="memory" />
      </Card>
    </div>
  {/if}
</div>

<style>
  .note { margin: 12px 0 0; font-size: 12px; max-width: 70ch; }
</style>
