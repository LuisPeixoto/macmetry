<script lang="ts">
  import { live } from "../../stores/live.svelte.ts";
  import { nav } from "../../stores/nav.svelte.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { t } from "../../stores/i18n.svelte.ts";
  import { C } from "../../colors.ts";
  import type { ThermalPressure } from "../../types.ts";
  import Card from "../ui/Card.svelte";
  import GaugeCard from "../ui/GaugeCard.svelte";
  import StatTile from "../ui/StatTile.svelte";
  import LineChart from "../charts/LineChart.svelte";
  import ViewHeader from "../layout/ViewHeader.svelte";

  const s = $derived(live.snapshot);
  const series = $derived(live.series);
  const startup = $derived(s?.disks?.volumes.find((v) => v.mount_point === "/") ?? s?.disks?.volumes[0]);
  const gpu = $derived(s?.gpus[0]);
  const net = $derived(s?.network?.interfaces.filter((i) => i.kind !== "loopback") ?? []);
  const diskIo = $derived(s?.disks?.io ?? []);
  const hasHeat = $derived(!!(s?.sensors || s?.power || s?.battery));

  const thermalTone = (p: ThermalPressure): "good" | "warn" | "bad" => (p === "nominal" ? "good" : p === "moderate" || p === "sleeping" ? "warn" : "bad");
  const sum = (f: (i: (typeof diskIo)[number]) => number) => diskIo.reduce((a, d) => a + f(d), 0);
  const memPressureName = $derived(s?.memory ? t(`pressure.${s.memory.pressure}`) : "–");
  const timesNow = $derived(live.times);
</script>

<ViewHeader title={t("nav.overview")} subtitle={t("overview.subtitle")}>
  {#snippet actions()}
    {#if s?.thermal_pressure}
      <span class="pill {thermalTone(s.thermal_pressure)}"><i class="dot"></i>{t("thermal.title")}: {t(`thermal.${s.thermal_pressure}`)}</span>
    {/if}
  {/snippet}
</ViewHeader>

<div class="scroll">
  <div class="stack">
    <div class="grid gauges">
      {#if s?.cpu}
        <GaugeCard title={t("nav.cpu")} icon="cpu" value={s.cpu.total_pct} big={fmt.percent(s.cpu.total_pct)} color={C.cpu}
          hint="{t('overview.load')} {fmt.num(s.cpu.load_average[0], 2)}" hint2={s.cpu.frequency_mhz ? fmt.freq(s.cpu.frequency_mhz) : undefined}
          spark={series["cpu.total"] ?? []} onopen={() => nav.go("cpu")} />
      {/if}
      {#if s?.memory}
        {@const pct = s.memory.total ? (s.memory.used / s.memory.total) * 100 : 0}
        <GaugeCard title={t("nav.memory")} icon="memory" value={pct} big={fmt.percent(pct)} color={C.memory}
          hint="{fmt.bytes(s.memory.used)} / {fmt.bytes(s.memory.total)}" hint2="{t('overview.pressure')}: {memPressureName}"
          spark={series["memory.used_pct"] ?? []} onopen={() => nav.go("memory")} />
      {/if}
      {#if gpu}
        <GaugeCard title={t("nav.gpu")} icon="gpu" value={gpu.utilization_pct} big={fmt.percent(gpu.utilization_pct)} color={C.gpu}
          hint={gpu.name} hint2={gpu.memory_used !== null ? (gpu.memory_total ? t("overview.vram", { used: fmt.bytes(gpu.memory_used), total: fmt.bytes(gpu.memory_total) }) : fmt.bytes(gpu.memory_used)) : undefined}
          spark={series["gpu.utilization"] ?? []} onopen={() => nav.go("gpu")} />
      {/if}
      {#if startup}
        {@const used = startup.total ? ((startup.total - startup.free) / startup.total) * 100 : 0}
        <GaugeCard title={t("nav.disk")} icon="disk" value={used} big={fmt.percent(used)} color={C.diskRead} warn={false}
          hint={t("overview.free_of", { free: fmt.disk(startup.free), total: fmt.disk(startup.total) })}
          hint2="{t('overview.read')} {fmt.rate(sum((d) => d.read_bps))} · {t('overview.write')} {fmt.rate(sum((d) => d.write_bps))}"
          spark={series["disk.read_bps"] ?? []} onopen={() => nav.go("disk")} />
      {/if}
    </div>

    <div class="grid two">
      <Card title={t("overview.network")} icon="network">
        {#snippet actions()}
          <span class="row sub num"><i class="chip" style="background:{C.netDown}"></i>{t("overview.download")} <b>{fmt.rate(net.reduce((a, i) => a + i.rx_bps, 0))}</b></span>
          <span class="row sub num"><i class="chip" style="background:{C.netUp}"></i>{t("overview.upload")} <b>{fmt.rate(net.reduce((a, i) => a + i.tx_bps, 0))}</b></span>
        {/snippet}
        <LineChart times={timesNow} height={170} unit="bytes_per_second" legend={false} table={false} label={t("overview.network")}
          series={[
            { key: "rx", label: t("overview.download"), color: C.netDown, values: series["net.rx_bps"] ?? [] },
            { key: "tx", label: t("overview.upload"), color: C.netUp, values: series["net.tx_bps"] ?? [] },
          ]} />
      </Card>

      {#if hasHeat}
        <Card title={t("overview.heat")} icon="sensors">
          <div class="tiles">
            {#if s?.sensors?.temperatures.length}
              {@const hot = Math.max(...s.sensors.temperatures.map((x) => x.celsius))}
              <StatTile label={t("overview.hottest")} value={fmt.temp(hot)} tone={hot >= 90 ? "bad" : hot >= 80 ? "warn" : "good"} />
            {/if}
            {#if s?.sensors?.fans.length}
              <StatTile label={t("overview.fan")} value={fmt.rpm(Math.max(...s.sensors.fans.map((x) => x.rpm)))} />
            {/if}
            {#if s?.power?.cpu_w != null}
              <StatTile label={t("overview.cpu_power")} value={fmt.watts(s.power.cpu_w)} hint={s.power.system_w != null ? `${t("metric.power.system")} ${fmt.watts(s.power.system_w)}` : undefined} />
            {/if}
            {#if s?.battery}
              <StatTile label={t("overview.battery")} value={fmt.percent(s.battery.percent)} hint={s.battery.minutes_remaining ? fmt.minutes(s.battery.minutes_remaining) : undefined} />
            {/if}
          </div>
          <div class="spark2">
            <LineChart times={timesNow} height={92} unit="celsius" legend={false} table={false} fill={false} label={t("metric.temp.max")}
              series={[{ key: "t", label: t("metric.temp.max"), color: C.temperature, values: series["temp.max"] ?? [] }]} />
          </div>
        </Card>
      {/if}
    </div>

    {#if live.processes.length}
      <Card title={t("overview.top_processes")} icon="processes">
        {#snippet actions()}<button type="button" class="link" onclick={() => nav.go("processes")}>{t("overview.see_all")}</button>{/snippet}
        <ul class="procs">
          {#each live.processes.slice(0, 5) as p (p.pid)}
            <li>
              <span class="pname">{p.name}</span>
              <span class="bar"><i style="width:{Math.min(100, p.cpu_pct)}%;background:{C.cpu}"></i></span>
              <b class="num">{fmt.num(p.cpu_pct, 1)}%</b>
              <span class="sub num mem">{fmt.bytes(p.memory_bytes)}</span>
            </li>
          {/each}
        </ul>
      </Card>
    {/if}
  </div>
</div>

<style>
  .scroll { overflow-y: auto; padding: 0 28px 28px; flex: 1; min-height: 0; }
  .gauges { grid-template-columns: repeat(auto-fit, minmax(205px, 1fr)); }
  .two { grid-template-columns: minmax(0, 1.6fr) minmax(0, 1fr); }
  @media (max-width: 1100px) { .two { grid-template-columns: 1fr; } }
  .tiles { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
  .spark2 { margin-top: 14px; }
  .chip { display: inline-block; width: 9px; height: 9px; border-radius: 3px; }
  .row b { color: var(--text); font-weight: 600; margin-left: 2px; }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: currentColor; display: inline-block; }
  .link { border: 0; background: transparent; color: var(--accent); cursor: pointer; font-weight: 550; padding: 0; }
  .link:hover { text-decoration: underline; }
  .procs { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 9px; }
  .procs li { display: grid; grid-template-columns: minmax(120px, 1.2fr) minmax(80px, 3fr) 64px 84px; align-items: center; gap: 14px; }
  .pname { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bar { height: 6px; background: var(--grid); border-radius: 4px; overflow: hidden; }
  .bar i { display: block; height: 100%; border-radius: 4px; }
  .procs b { text-align: right; font-weight: 600; }
  .mem { text-align: right; }
</style>
