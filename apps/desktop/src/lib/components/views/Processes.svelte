<script lang="ts">
  import { live } from "../../stores/live.svelte.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { t } from "../../stores/i18n.svelte.ts";
  import { C } from "../../colors.ts";
  import { defaultDirection, filterProcesses, sortProcesses, type SortKey } from "../../processes.ts";
  import Card from "../ui/Card.svelte";
  import Icon from "../ui/Icon.svelte";
  import EmptyState from "../ui/EmptyState.svelte";
  import ViewHeader from "../layout/ViewHeader.svelte";

  let search = $state("");
  let key = $state<SortKey>("cpu");
  let direction = $state<"asc" | "desc">("desc");

  const rows = $derived(sortProcesses(filterProcesses(live.processes, search), key, direction));
  // The bars are relative to the busiest row on screen, so they stay readable when the whole Mac is quiet.
  const topCpu = $derived(Math.max(5, ...rows.map((p) => p.cpu_pct)));

  function sortBy(next: SortKey) {
    if (next === key) direction = direction === "asc" ? "desc" : "asc";
    else { key = next; direction = defaultDirection(next); }
  }
  const columns: { key: SortKey; label: string; right?: boolean }[] = [
    { key: "name", label: "proc.name" }, { key: "pid", label: "proc.pid", right: true }, { key: "user", label: "proc.user" },
    { key: "cpu", label: "proc.cpu", right: true }, { key: "memory", label: "proc.memory", right: true }, { key: "threads", label: "proc.threads", right: true },
    { key: "read", label: "proc.read", right: true }, { key: "write", label: "proc.write", right: true },
  ];
</script>

<ViewHeader title={t("nav.processes")} subtitle={t("proc.subtitle", { n: rows.length })}>
  {#snippet actions()}
    <label class="search">
      <Icon name="search" size={14} />
      <input type="search" placeholder={t("proc.search")} bind:value={search} aria-label={t("proc.search")} spellcheck="false" autocomplete="off" />
    </label>
  {/snippet}
</ViewHeader>

<div class="scroll">
  {#if !live.processes.length}
    <EmptyState icon="processes" title={t("proc.waiting")} />
  {:else}
    <Card pad={false}>
      <div class="wrap">
        <table class="table">
          <thead>
            <tr>
              {#each columns as c (c.key)}
                <th class:r={c.right} aria-sort={key === c.key ? (direction === "asc" ? "ascending" : "descending") : "none"}>
                  <button type="button" class:on={key === c.key} onclick={() => sortBy(c.key)}>
                    {t(c.label)}{#if key === c.key}<span aria-hidden="true">{direction === "asc" ? " ↑" : " ↓"}</span>{/if}
                  </button>
                </th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each rows as p (p.pid)}
              <tr>
                <td class="name" title={p.name}>{p.name}</td>
                <td class="r num muted">{p.pid}</td>
                <td class="muted">{p.user}</td>
                <td class="r num"><span class="cpu"><i style="width:{Math.min(100, (p.cpu_pct / topCpu) * 100)}%;background:{C.cpu}"></i><b>{fmt.num(p.cpu_pct, 1)}%</b></span></td>
                <td class="r num">{fmt.bytes(p.memory_bytes)}</td>
                <td class="r num muted">{p.threads}</td>
                <td class="r num muted">{p.disk_read_bps > 0 ? fmt.rate(p.disk_read_bps) : "–"}</td>
                <td class="r num muted">{p.disk_write_bps > 0 ? fmt.rate(p.disk_write_bps) : "–"}</td>
              </tr>
            {/each}
          </tbody>
        </table>
        {#if !rows.length}<p class="none muted">{t("proc.no_match")}</p>{/if}
      </div>
    </Card>
    <p class="note muted">{t("proc.note")}</p>
  {/if}
</div>

<style>
  .wrap { overflow: auto; max-height: calc(100vh - 150px); padding: 0 18px 8px; }
  thead th { position: sticky; top: 0; background: var(--card); padding-top: 14px; z-index: 1; }
  th button { all: unset; cursor: pointer; font: inherit; color: inherit; text-transform: inherit; letter-spacing: inherit; }
  th button:hover, th button.on { color: var(--text); }
  .search { display: flex; align-items: center; gap: 8px; padding: 0 10px; height: 30px; border-radius: 9px; background: var(--card); border: 1px solid var(--line); color: var(--text-3); min-width: 220px; }
  .search:focus-within { border-color: var(--accent); }
  input { all: unset; flex: 1; color: var(--text); font-size: 12.5px; user-select: text; -webkit-user-select: text; }
  .cpu { display: block; position: relative; min-width: 130px; padding: 2px 8px; text-align: right; }
  .cpu i { position: absolute; inset: 0 auto 0 0; border-radius: 5px; opacity: 0.2; }
  .cpu b { position: relative; font-weight: 600; }
  .none { text-align: center; padding: 28px 0 22px; margin: 0; }
  .note { font-size: 12px; margin: 12px 4px 0; }
</style>
