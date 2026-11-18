<script lang="ts">
  // One line of the menu bar popover: what it is, its value, and a sparkline of the last few minutes.
  import Spark from "../charts/Spark.svelte";
  import Icon from "./Icon.svelte";
  import type { Value } from "../../chart.ts";

  let { icon, label, value, hint, spark, color, max }: { icon: string; label: string; value: string; hint?: string; spark?: Value[]; color: string; max?: number } = $props();
</script>

<div class="row" title={hint}>
  <span class="ico" style="color:{color}"><Icon name={icon as never} size={16} /></span>
  <div class="label">{label}</div>
  {#if spark}<div class="spark"><Spark values={spark} {color} height={22} {max} /></div>{/if}
  <b class="num">{value}</b>
</div>

<style>
  .row { display: grid; grid-template-columns: 20px minmax(70px, 1fr) 84px 74px; align-items: center; gap: 10px; padding: 7px 12px; border-radius: 11px; background: color-mix(in srgb, var(--text) 5%, transparent); }
  .ico { display: grid; place-items: center; }
  .label { font-weight: 560; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  b { text-align: right; font-weight: 650; font-size: 14px; }
</style>
