<script lang="ts">
  import { live } from "../../stores/live.svelte.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { t } from "../../stores/i18n.svelte.ts";
  import { C } from "../../colors.ts";
  import Card from "../ui/Card.svelte";
  import StatTile from "../ui/StatTile.svelte";
  import LineChart from "../charts/LineChart.svelte";
  import ViewHeader from "../layout/ViewHeader.svelte";

  const d = $derived(live.snapshot?.disks);
  const series = $derived(live.series);
  const read = $derived(d?.io.reduce((a, x) => a + x.read_bps, 0) ?? 0);
  const write = $derived(d?.io.reduce((a, x) => a + x.write_bps, 0) ?? 0);
  const ops = $derived(d?.io.reduce((a, x) => a + x.read_ops + x.write_ops, 0) ?? 0);
</script>

<ViewHeader title={t("nav.disk")} subtitle={t("disk.subtitle", { n: d?.volumes.length ?? 0 })} />

<div class="scroll">
  {#if d}
    <div class="stack">
      <Card>
        <div class="tiles">
          <StatTile label={t("overview.read")} value={fmt.rate(read)} />
          <StatTile label={t("overview.write")} value={fmt.rate(write)} />
          <StatTile label={t("disk.ops")} value="{fmt.num(ops, 0)}/s" />
        </div>
      </Card>

      <Card title={t("disk.volumes")} icon="disk">
        <ul class="vols">
          {#each d.volumes as v (v.mount_point)}
            {@const used = v.total - v.free}
            {@const pct = v.total ? (used / v.total) * 100 : 0}
            <li>
              <div class="line">
                <b>{v.name}</b>
                <span class="muted">{v.mount_point} · {v.fs_type}</span>
                <span class="sub num r">{t("overview.free_of", { free: fmt.disk(v.free), total: fmt.disk(v.total) })}</span>
              </div>
              <div class="meter"><i style="width:{pct}%;background:{pct >= 90 ? 'var(--bad)' : pct >= 75 ? 'var(--warn)' : 'var(--accent)'}"></i></div>
            </li>
          {/each}
        </ul>
      </Card>

      <Card title={t("disk.throughput")} icon="disk">
        <LineChart times={live.times} height={220} unit="bytes_per_second" label={t("disk.throughput")}
          series={[
            { key: "r", label: t("overview.read"), color: C.diskRead, values: series["disk.read_bps"] ?? [] },
            { key: "w", label: t("overview.write"), color: C.diskWrite, values: series["disk.write_bps"] ?? [] },
          ]} />
      </Card>

      {#if d.io.length}
        <Card title={t("disk.physical")} icon="disk">
          <table class="table">
            <thead><tr><th>{t("disk.device")}</th><th class="r">{t("overview.read")}</th><th class="r">{t("overview.write")}</th><th class="r">{t("disk.ops")}</th></tr></thead>
            <tbody>
              {#each d.io as x (x.name)}
                <tr><td>{x.name}</td><td class="r num">{fmt.rate(x.read_bps)}</td><td class="r num">{fmt.rate(x.write_bps)}</td><td class="r num">{fmt.num(x.read_ops + x.write_ops, 0)}/s</td></tr>
              {/each}
            </tbody>
          </table>
        </Card>
      {/if}
    </div>
  {/if}
</div>

<style>
  .vols { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 16px; }
  .line { display: flex; align-items: baseline; gap: 10px; margin-bottom: 7px; flex-wrap: wrap; }
  .r { margin-left: auto; }
</style>
