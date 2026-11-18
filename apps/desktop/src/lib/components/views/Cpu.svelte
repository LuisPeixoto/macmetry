<script lang="ts">
  import { live } from "../../stores/live.svelte.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { t } from "../../stores/i18n.svelte.ts";
  import { C, tone } from "../../colors.ts";
  import { loadLevel } from "../../chart.ts";
  import Card from "../ui/Card.svelte";
  import StatTile from "../ui/StatTile.svelte";
  import ProcessList from "../ui/ProcessList.svelte";
  import LineChart from "../charts/LineChart.svelte";
  import CoreBars from "../charts/CoreBars.svelte";
  import ViewHeader from "../layout/ViewHeader.svelte";

  const cpu = $derived(live.snapshot?.cpu);
  const sys = $derived(live.info?.system);
  const series = $derived(live.series);
  const subtitle = $derived(
    sys?.performance_cores != null
      ? t("cpu.subtitle_silicon", { p: sys.performance_cores, e: sys.efficiency_cores ?? 0 })
      : t("cpu.subtitle_intel", { cores: sys?.physical_cores ?? 0, threads: sys?.logical_cores ?? 0 }),
  );
  const groups = $derived.by(() => {
    if (!cpu) return [];
    const kinds = [...new Set(cpu.cores.map((c) => c.kind))];
    if (kinds.length <= 1) return [{ title: t("cpu.cores"), cores: cpu.cores, cluster: null }];
    return (["performance", "efficiency"] as const)
      .filter((k) => kinds.includes(k))
      .map((k) => ({
        title: t(`cpu.${k}`),
        cores: cpu.cores.filter((c) => c.kind === k),
        cluster: cpu.clusters.find((c) => c.name.toLowerCase() === k) ?? null,
      }));
  });
</script>

<ViewHeader title={t("nav.cpu")} {subtitle} />

<div class="scroll">
  {#if cpu}
    <div class="stack">
      <Card>
        <div class="tiles">
          <StatTile label={t("cpu.usage")} value={fmt.percent(cpu.total_pct, 1)} tone={tone(loadLevel(cpu.total_pct))} hint="{t('cpu.user')} {fmt.percent(cpu.user_pct, 1)} · {t('cpu.system')} {fmt.percent(cpu.system_pct, 1)}" />
          <StatTile label={t("cpu.load")} value={fmt.num(cpu.load_average[0], 2)} hint="{fmt.num(cpu.load_average[0], 2)} · {fmt.num(cpu.load_average[1], 2)} · {fmt.num(cpu.load_average[2], 2)} ({t('cpu.load_hint')})" />
          {#if cpu.frequency_mhz}
            <StatTile label={t("cpu.frequency")} value={fmt.freq(cpu.frequency_mhz)} hint={cpu.frequency_is_measured ? t("cpu.freq_measured") : t("cpu.freq_nominal")} />
          {/if}
          {#if live.snapshot?.thermal_pressure}
            <StatTile label={t("thermal.title")} value={t(`thermal.${live.snapshot.thermal_pressure}`)} tone={live.snapshot.thermal_pressure === "nominal" ? "good" : live.snapshot.thermal_pressure === "moderate" ? "warn" : "bad"} />
          {/if}
        </div>
      </Card>

      <Card title={t("cpu.history")} icon="cpu">
        <LineChart times={live.times} height={230} unit="percent" label={t("cpu.history")}
          series={[
            { key: "total", label: t("cpu.usage"), color: C.cpu, values: series["cpu.total"] ?? [] },
            { key: "system", label: t("cpu.system"), color: C.cpuSystem, values: series["cpu.system"] ?? [] },
          ]} />
      </Card>

      {#each groups as g (g.title)}
        <Card title={g.title} icon="cpu">
          {#snippet actions()}
            {#if g.cluster}
              <span class="sub num">{t("cpu.usage")} <b>{fmt.percent(g.cluster.usage_pct, 1)}</b>{#if g.cluster.frequency_mhz} · {fmt.freq(g.cluster.frequency_mhz)}{/if}</span>
            {/if}
          {/snippet}
          <CoreBars cores={g.cores} />
        </Card>
      {/each}

      <Card title={t("cpu.busiest")} icon="processes">
        <ProcessList processes={live.processes} by="cpu" />
      </Card>
    </div>
  {/if}
</div>

<style>
  b { color: var(--text); font-weight: 600; }
</style>
