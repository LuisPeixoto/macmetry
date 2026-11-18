<script lang="ts">
  import { live } from "../../stores/live.svelte.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { t } from "../../stores/i18n.svelte.ts";
  import { C } from "../../colors.ts";
  import Card from "../ui/Card.svelte";
  import StatTile from "../ui/StatTile.svelte";
  import LineChart from "../charts/LineChart.svelte";
  import ViewHeader from "../layout/ViewHeader.svelte";

  const all = $derived(live.snapshot?.network?.interfaces ?? []);
  const external = $derived(all.filter((i) => i.kind !== "loopback"));
  // Quiet virtual interfaces (VPN tunnels, bridges) would bury the ones that carry traffic, so they sit behind the active ones.
  const rows = $derived([...all].sort((a, b) => Number(b.up) - Number(a.up) || b.rx_bps + b.tx_bps - (a.rx_bps + a.tx_bps) || a.name.localeCompare(b.name)));
  const series = $derived(live.series);
  const active = $derived(external.filter((i) => i.up && i.kind !== "virtual").length);
</script>

<ViewHeader title={t("nav.network")} subtitle={t("net.subtitle", { n: active })} />

<div class="scroll">
  <div class="stack">
    <Card>
      <div class="tiles">
        <StatTile label={t("overview.download")} value={fmt.rate(external.reduce((a, i) => a + i.rx_bps, 0))} hint="{t('net.total_in')} {fmt.disk(external.reduce((a, i) => a + i.rx_total, 0))}" />
        <StatTile label={t("overview.upload")} value={fmt.rate(external.reduce((a, i) => a + i.tx_bps, 0))} hint="{t('net.total_out')} {fmt.disk(external.reduce((a, i) => a + i.tx_total, 0))}" />
        <StatTile label={t("net.packets")} value="{fmt.num(external.reduce((a, i) => a + i.rx_packets_ps + i.tx_packets_ps, 0), 0)}/s" />
        <StatTile label={t("net.errors")} value={fmt.num(external.reduce((a, i) => a + i.rx_errors + i.tx_errors, 0), 0)} tone={external.some((i) => i.rx_errors + i.tx_errors > 0) ? "warn" : "good"} />
      </div>
    </Card>

    <Card title={t("net.speed")} icon="network">
      <LineChart times={live.times} height={230} unit="bytes_per_second" label={t("net.speed")}
        series={[
          { key: "rx", label: t("overview.download"), color: C.netDown, values: series["net.rx_bps"] ?? [] },
          { key: "tx", label: t("overview.upload"), color: C.netUp, values: series["net.tx_bps"] ?? [] },
        ]} />
    </Card>

    <Card title={t("net.interfaces")} icon="network" pad={false}>
      <div class="tablewrap">
        <table class="table">
          <thead><tr><th>{t("net.interface")}</th><th>{t("net.kind")}</th><th class="r">{t("overview.download")}</th><th class="r">{t("overview.upload")}</th><th class="r">{t("net.total_in")}</th><th class="r">{t("net.total_out")}</th><th>{t("net.addresses")}</th></tr></thead>
          <tbody>
            {#each rows as i (i.name)}
              <tr class:quiet={!i.up}>
                <td><b>{i.name}</b></td>
                <td>{t(`net.kind.${i.kind}`)}{#if !i.up} <span class="muted">· {t("net.down")}</span>{/if}</td>
                <td class="r num">{fmt.rate(i.rx_bps)}</td><td class="r num">{fmt.rate(i.tx_bps)}</td>
                <td class="r num">{fmt.disk(i.rx_total)}</td><td class="r num">{fmt.disk(i.tx_total)}</td>
                <td class="sub addr">{i.addresses.length ? i.addresses.join(", ") : "–"}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </Card>
  </div>
</div>

<style>
  .tablewrap { overflow-x: auto; padding: 12px 18px 16px; }
  .table th, .table td { padding-right: 16px; }
  .quiet { opacity: 0.55; }
  .addr { max-width: 280px; overflow: hidden; text-overflow: ellipsis; user-select: text; -webkit-user-select: text; }
</style>
