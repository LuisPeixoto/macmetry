<script lang="ts">
  import { live } from "../../stores/live.svelte.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { t } from "../../stores/i18n.svelte.ts";
  import { C } from "../../colors.ts";
  import type { SensorKind, Temperature } from "../../types.ts";
  import Card from "../ui/Card.svelte";
  import StatTile from "../ui/StatTile.svelte";
  import EmptyState from "../ui/EmptyState.svelte";
  import LineChart from "../charts/LineChart.svelte";
  import StackedBar from "../charts/StackedBar.svelte";
  import ViewHeader from "../layout/ViewHeader.svelte";

  const s = $derived(live.snapshot);
  const series = $derived(live.series);
  const KINDS: SensorKind[] = ["cpu", "gpu", "memory", "storage", "battery", "ambient", "power", "other"];
  const groups = $derived(
    KINDS.map((kind) => ({ kind, items: (s?.sensors?.temperatures ?? []).filter((x) => x.kind === kind).sort((a, b) => b.celsius - a.celsius) })).filter((g) => g.items.length),
  );
  const heat = (c: number) => (c >= 95 ? "var(--bad)" : c >= 80 ? "var(--warn)" : "var(--s8)");
  const hottest = $derived(s?.sensors?.temperatures.reduce<Temperature | null>((a, x) => (!a || x.celsius > a.celsius ? x : a), null) ?? null);
  const power = $derived(s?.power);
  const powerSegments = $derived(
    power
      ? ([["cpu", power.cpu_w, "var(--s1)"], ["gpu", power.gpu_w, "var(--s3)"], ["ane", power.ane_w, "var(--s7)"], ["dram", power.dram_w, "var(--s4)"]] as const)
          .filter(([, w]) => w != null && w > 0)
          .map(([k, w, color]) => ({ label: t(`sensors.power.${k}`), value: w as number, color, text: fmt.watts(w) }))
      : [],
  );
  const nothing = $derived(!s?.sensors && !s?.power && !s?.thermal_pressure);
</script>

<ViewHeader title={t("nav.sensors")} subtitle={t("sensors.subtitle")} />

<div class="scroll">
  {#if nothing}
    <EmptyState icon="sensors" title={t("sensors.none")} text={t("sensors.none_text")} />
  {:else}
    <div class="stack">
      <Card>
        <div class="tiles">
          {#if s?.thermal_pressure}
            <StatTile label={t("thermal.title")} value={t(`thermal.${s.thermal_pressure}`)} tone={s.thermal_pressure === "nominal" ? "good" : s.thermal_pressure === "moderate" ? "warn" : "bad"} hint={t(`sensors.thermal_hint.${s.thermal_pressure}`)} />
          {/if}
          {#if hottest}<StatTile label={t("overview.hottest")} value={fmt.temp(hottest.celsius)} hint={hottest.label} tone={hottest.celsius >= 95 ? "bad" : hottest.celsius >= 80 ? "warn" : "good"} />{/if}
          {#if s?.sensors?.fans.length}<StatTile label={t("overview.fan")} value={fmt.rpm(Math.max(...s.sensors.fans.map((f) => f.rpm)))} />{/if}
          {#if power?.cpu_w != null}<StatTile label={t("overview.cpu_power")} value={fmt.watts(power.cpu_w)} />{/if}
          {#if power?.system_w != null}<StatTile label={t("metric.power.system")} value={fmt.watts(power.system_w)} />{/if}
        </div>
      </Card>

      {#if (series["temp.max"] ?? []).some((v) => v !== null)}
        <Card title={t("sensors.history")} icon="sensors">
          <LineChart times={live.times} height={200} unit="celsius" label={t("sensors.history")}
            series={[
              { key: "max", label: t("metric.temp.max"), color: C.temperature, values: series["temp.max"] ?? [] },
              ...((series["temp.cpu"] ?? []).some((v) => v !== null) ? [{ key: "cpu", label: t("metric.temp.cpu"), color: C.cpu, values: series["temp.cpu"] ?? [] }] : []),
            ]} />
        </Card>
      {/if}

      {#if s?.sensors?.fans.length}
        <Card title={t("sensors.fans")} icon="fan">
          <ul class="list">
            {#each s.sensors.fans as f (f.index)}
              <li>
                <span>{f.label}</span>
                <span class="meter"><i style="width:{f.max_rpm ? Math.min(100, (f.rpm / f.max_rpm) * 100) : 0}%;background:{C.fan}"></i></span>
                <b class="num">{fmt.rpm(f.rpm)}</b>
                <span class="muted num">{f.max_rpm ? `max ${fmt.rpm(f.max_rpm)}` : ""}</span>
              </li>
            {/each}
          </ul>
        </Card>
      {/if}

      {#if powerSegments.length}
        <Card title={t("sensors.power")} icon="bolt">
          <StackedBar segments={powerSegments} />
        </Card>
      {/if}

      {#each groups as g (g.kind)}
        <Card title={t(`sensors.kind.${g.kind}`)} icon="sensors">
          <ul class="list">
            {#each g.items as x (x.key)}
              <li>
                <span class="name" title={x.key}>{x.label}</span>
                <span class="meter"><i style="width:{Math.min(100, (x.celsius / 110) * 100)}%;background:{heat(x.celsius)}"></i></span>
                <b class="num">{fmt.temp(x.celsius)}</b>
                <span></span>
              </li>
            {/each}
          </ul>
        </Card>
      {/each}
    </div>
  {/if}
</div>

<style>
  .list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 10px; }
  li { display: grid; grid-template-columns: minmax(130px, 1.2fr) minmax(80px, 3fr) 80px 110px; align-items: center; gap: 14px; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  b { text-align: right; font-weight: 600; }
  .muted { text-align: right; font-size: 12px; }
</style>
