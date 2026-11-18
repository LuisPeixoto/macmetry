<script lang="ts">
  import { live } from "../../stores/live.svelte.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { t } from "../../stores/i18n.svelte.ts";
  import { C, tone } from "../../colors.ts";
  import { loadLevel } from "../../chart.ts";
  import Card from "../ui/Card.svelte";
  import StatTile from "../ui/StatTile.svelte";
  import LineChart from "../charts/LineChart.svelte";
  import ViewHeader from "../layout/ViewHeader.svelte";

  const gpus = $derived(live.snapshot?.gpus ?? []);
  const series = $derived(live.series);
</script>

<ViewHeader title={t("nav.gpu")} subtitle={gpus.map((g) => g.name).join(" · ")} />

<div class="scroll">
  <div class="stack">
    {#each gpus as g (g.name)}
      <Card title={g.name} icon="gpu">
        <div class="tiles">
          <StatTile label={t("gpu.usage")} value={fmt.percent(g.utilization_pct, 0)} tone={g.utilization_pct == null ? undefined : tone(loadLevel(g.utilization_pct))} />
          {#if g.memory_used != null}
            <StatTile label={t("gpu.memory")} value={fmt.bytes(g.memory_used)} hint={g.memory_total ? t("overview.vram", { used: fmt.percent((g.memory_used / g.memory_total) * 100, 0), total: fmt.bytes(g.memory_total) }) : t("gpu.shared")} />
          {/if}
          {#if g.core_clock_mhz != null}<StatTile label={t("gpu.core_clock")} value={fmt.freq(g.core_clock_mhz)} />{/if}
          {#if g.memory_clock_mhz != null}<StatTile label={t("gpu.memory_clock")} value={fmt.freq(g.memory_clock_mhz)} />{/if}
          {#if g.temperature_c != null}<StatTile label={t("gpu.temperature")} value={fmt.temp(g.temperature_c)} tone={g.temperature_c >= 90 ? "bad" : g.temperature_c >= 80 ? "warn" : "good"} />{/if}
          {#if g.power_w != null}<StatTile label={t("gpu.power")} value={fmt.watts(g.power_w)} />{/if}
          {#if g.fan_rpm != null}<StatTile label={t("gpu.fan")} value={fmt.rpm(g.fan_rpm)} />{/if}
        </div>
      </Card>
    {/each}

    {#if gpus.length}
      <Card title={t("gpu.history")} icon="gpu">
        <LineChart times={live.times} height={220} unit="percent" legend={false} label={t("gpu.history")}
          series={[{ key: "gpu", label: t("gpu.usage"), color: C.gpu, values: series["gpu.utilization"] ?? [] }]} />
      </Card>
      {#if (series["gpu.temperature"] ?? []).some((v) => v !== null)}
        <Card title={t("gpu.temperature")} icon="sensors">
          <LineChart times={live.times} height={150} unit="celsius" legend={false} table={false} fill={false} label={t("gpu.temperature")}
            series={[{ key: "gt", label: t("gpu.temperature"), color: C.temperature, values: series["gpu.temperature"] ?? [] }]} />
        </Card>
      {/if}
    {/if}
  </div>
</div>
