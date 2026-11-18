<script lang="ts">
  import { live } from "../../stores/live.svelte.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { t } from "../../stores/i18n.svelte.ts";
  import { C } from "../../colors.ts";
  import Card from "../ui/Card.svelte";
  import StatTile from "../ui/StatTile.svelte";
  import EmptyState from "../ui/EmptyState.svelte";
  import Gauge from "../charts/Gauge.svelte";
  import LineChart from "../charts/LineChart.svelte";
  import ViewHeader from "../layout/ViewHeader.svelte";

  const b = $derived(live.snapshot?.battery);
  const series = $derived(live.series);
  // For a battery the bad end is empty: amber below 20% and red below 10%, unless it is on the charger.
  const level = $derived(!b || b.on_ac_power ? "ok" : b.percent < 10 ? "hot" : b.percent < 20 ? "busy" : "ok");
</script>

<ViewHeader title={t("nav.battery")} subtitle={b ? t(`battery.state.${b.state}`) : ""} />

<div class="scroll">
  {#if !b}
    <EmptyState icon="battery" title={t("battery.none")} text={t("battery.none_text")} />
  {:else}
    <div class="stack">
      <Card>
        <div class="hero">
          <Gauge value={b.percent} size={150} stroke={14} {level}><div class="big">{fmt.percent(b.percent)}</div></Gauge>
          <div class="tiles">
            <StatTile label={t("battery.status")} value={t(`battery.state.${b.state}`)} hint={b.on_ac_power ? t("battery.plugged") : t("battery.unplugged")} tone={b.percent < 15 && !b.on_ac_power ? "bad" : undefined} />
            {#if b.minutes_remaining}<StatTile label={b.state === "charging" ? t("battery.to_full") : t("battery.remaining")} value={fmt.minutes(b.minutes_remaining)} />{/if}
            {#if b.health_pct != null}<StatTile label={t("battery.health")} value={fmt.percent(b.health_pct)} hint={t("battery.health_hint")} tone={b.health_pct < 80 ? "warn" : "good"} />{/if}
            {#if b.cycle_count != null}<StatTile label={t("battery.cycles")} value={fmt.num(b.cycle_count)} />{/if}
            {#if b.power_w != null}<StatTile label={t("battery.power")} value={fmt.watts(Math.abs(b.power_w))} hint={b.power_w < 0 ? t("battery.draining") : t("battery.charging_power")} />{/if}
            {#if b.temperature_c != null}<StatTile label={t("battery.temperature")} value={fmt.temp(b.temperature_c)} />{/if}
            {#if b.adapter_w != null}<StatTile label={t("battery.adapter")} value={fmt.watts(b.adapter_w)} />{/if}
            {#if b.voltage_v != null}<StatTile label={t("battery.voltage")} value="{fmt.num(b.voltage_v, 2)} V" />{/if}
          </div>
        </div>
      </Card>

      <Card title={t("battery.history")} icon="battery">
        <LineChart times={live.times} height={210} unit="percent" legend={false} label={t("battery.history")}
          series={[{ key: "bat", label: t("metric.battery.percent"), color: C.battery, values: series["battery.percent"] ?? [] }]} />
      </Card>
    </div>
  {/if}
</div>

<style>
  .hero { display: flex; align-items: center; gap: 36px; flex-wrap: wrap; }
  .hero .tiles { flex: 1; min-width: 300px; }
</style>
