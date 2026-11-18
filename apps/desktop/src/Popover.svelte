<script lang="ts">
  import { onMount } from "svelte";
  import { live } from "./lib/stores/live.svelte.ts";
  import { settings } from "./lib/stores/settings.svelte.ts";
  import { fmt } from "./lib/stores/fmt.svelte.ts";
  import { t } from "./lib/stores/i18n.svelte.ts";
  import { getBackend } from "./lib/api.ts";
  import { C } from "./lib/colors.ts";
  import PopRow from "./lib/components/ui/PopRow.svelte";
  import Icon from "./lib/components/ui/Icon.svelte";

  onMount(() => {
    settings.load();
    live.start();
    return () => live.dispose();
  });

  const s = $derived(live.snapshot);
  const series = $derived(live.series);
  const net = $derived(s?.network?.interfaces.filter((i) => i.kind !== "loopback") ?? []);
  const gpu = $derived(s?.gpus[0]);
  const startup = $derived(s?.disks?.volumes.find((v) => v.mount_point === "/") ?? s?.disks?.volumes[0]);
  const hottest = $derived(s?.sensors?.temperatures.length ? Math.max(...s.sensors.temperatures.map((x) => x.celsius)) : null);
  const memPct = $derived(s?.memory?.total ? (s.memory.used / s.memory.total) * 100 : 0);
  const open = async () => (await getBackend()).showMainWindow();
  const quit = async () => (await getBackend()).quitApp();
</script>

<main>
  <header data-tauri-drag-region>
    <div class="brand"><img src="/logo.svg" alt="" width="22" height="22" /><span>macmetry</span></div>
    {#if s?.thermal_pressure}
      <span class="pill {s.thermal_pressure === 'nominal' ? 'good' : s.thermal_pressure === 'moderate' ? 'warn' : 'bad'}">{t(`thermal.${s.thermal_pressure}`)}</span>
    {/if}
  </header>

  {#if s}
    <div class="rows">
      {#if s.cpu}<PopRow icon="cpu" label={t("nav.cpu")} value={fmt.percent(s.cpu.total_pct)} hint="{t('overview.load')} {fmt.num(s.cpu.load_average[0], 2)}" spark={series["cpu.total"]} color={C.cpu} max={100} />{/if}
      {#if s.memory}<PopRow icon="memory" label={t("nav.memory")} value={fmt.percent(memPct)} hint="{fmt.bytes(s.memory.used)} · {t(`pressure.${s.memory.pressure}`)}" spark={series["memory.used_pct"]} color={C.memory} max={100} />{/if}
      {#if gpu?.utilization_pct != null}<PopRow icon="gpu" label={t("nav.gpu")} value={fmt.percent(gpu.utilization_pct)} hint={gpu.name} spark={series["gpu.utilization"]} color={C.gpu} max={100} />{/if}
      {#if startup}<PopRow icon="disk" label={t("nav.disk")} value={fmt.disk(startup.free)} hint="{t('common.free')} · {t('overview.read')} {fmt.rate(s.disks?.io.reduce((a, d) => a + d.read_bps, 0))}" spark={series["disk.read_bps"]} color={C.diskRead} />{/if}
      {#if net.length}<PopRow icon="network" label={t("nav.network")} value={fmt.rate(net.reduce((a, i) => a + i.rx_bps, 0))} hint="↓ {t('overview.download')} · ↑ {fmt.rate(net.reduce((a, i) => a + i.tx_bps, 0))}" spark={series["net.rx_bps"]} color={C.netDown} />{/if}
      {#if hottest != null}<PopRow icon="sensors" label={t("pop.temp")} value={fmt.temp(hottest)} hint={s.sensors?.fans.length ? `${t("overview.fan")} ${fmt.rpm(Math.max(...s.sensors.fans.map((f) => f.rpm)))}` : undefined} spark={series["temp.max"]} color={C.temperature} />{/if}
      {#if s.power?.cpu_w != null}<PopRow icon="bolt" label={t("pop.power")} value={fmt.watts(s.power.cpu_w)} spark={series["power.cpu"]} color={C.power} />{/if}
      {#if s.battery}<PopRow icon="battery" label={t("nav.battery")} value={fmt.percent(s.battery.percent)} hint={s.battery.minutes_remaining ? fmt.minutes(s.battery.minutes_remaining) : t(`battery.state.${s.battery.state}`)} spark={series["battery.percent"]} color={C.battery} max={100} />{/if}
    </div>

    {#if live.processes.length}
      <div class="procs">
        <div class="eyebrow">{t("overview.top_processes")}</div>
        {#each live.processes.slice(0, 3) as p (p.pid)}
          <div class="proc"><span class="pname">{p.name}</span><b class="num">{fmt.num(p.cpu_pct, 1)}%</b></div>
        {/each}
      </div>
    {/if}
  {/if}

  <footer>
    <button type="button" class="primary" onclick={open}><Icon name="overview" size={14} />{t("pop.open")}</button>
    <button type="button" class="quiet" onclick={quit} title={t("pop.quit")}>{t("pop.quit")}</button>
  </footer>
</main>

<style>
  main { height: 100vh; display: flex; flex-direction: column; padding: 12px; gap: 10px; box-sizing: border-box; overflow: hidden; }
  header { display: flex; align-items: center; justify-content: space-between; padding: 2px 4px 0; }
  .brand { display: flex; align-items: center; gap: 8px; font-weight: 650; letter-spacing: -0.01em; }
  .brand img { border-radius: 6px; }
  .rows { display: flex; flex-direction: column; gap: 6px; flex: 1; min-height: 0; overflow: hidden; }
  .procs { padding: 8px 12px; border-radius: 12px; background: color-mix(in srgb, var(--text) 5%, transparent); display: flex; flex-direction: column; gap: 5px; }
  .proc { display: flex; justify-content: space-between; gap: 12px; font-size: 12.5px; }
  .pname { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  footer { display: flex; gap: 8px; align-items: center; }
  button { font: inherit; cursor: pointer; border-radius: 10px; height: 32px; border: 1px solid var(--line-2); }
  .primary { flex: 1; display: flex; align-items: center; justify-content: center; gap: 8px; background: var(--grad); color: #fff; border: 0; font-weight: 600; }
  .primary:hover { filter: brightness(1.08); }
  .quiet { padding: 0 14px; background: color-mix(in srgb, var(--text) 6%, transparent); color: var(--text-2); }
  .quiet:hover { color: var(--text); }
</style>
