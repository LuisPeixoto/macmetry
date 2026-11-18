<script lang="ts">
  import { availableViews, type ViewId } from "../../stores/nav.ts";
  import { nav } from "../../stores/nav.svelte.ts";
  import { live } from "../../stores/live.svelte.ts";
  import { settings } from "../../stores/settings.svelte.ts";
  import { t } from "../../stores/i18n.svelte.ts";
  import { chipLabel } from "../../format.ts";
  import Icon from "../ui/Icon.svelte";

  const items = $derived(availableViews(live.info?.capabilities ?? null));
  const groups = ["monitor", "data"] as const;
  const system = $derived(live.info?.system);
</script>

<aside class="sidebar">
  <div class="drag" data-tauri-drag-region></div>
  <div class="mac">
    <div class="name">{system?.model ?? "Mac"}</div>
    <div class="chip">{system ? chipLabel(system.chip, system.physical_cores) : ""}</div>
  </div>

  <nav>
    {#each groups as group (group)}
      <div class="group">{t(`nav.${group}`)}</div>
      {#each items.filter((i) => i.group === group) as item (item.id)}
        <button type="button" class="item" class:on={nav.view === item.id} aria-current={nav.view === item.id ? "page" : undefined} onclick={() => nav.go(item.id as ViewId)}>
          <Icon name={item.icon as never} size={17} />
          <span>{t(`nav.${item.id}`)}</span>
        </button>
      {/each}
    {/each}
  </nav>

  <div class="foot">
    <button type="button" class="item" class:on={nav.view === "settings"} onclick={() => nav.go("settings")}>
      <Icon name="settings" size={17} /><span>{t("nav.settings")}</span>
    </button>
    <div class="status">
      <span class="dot" class:off={!settings.value.record_history}></span>
      <span>{settings.value.record_history ? t("status.recording") : t("status.not_recording")}</span>
      <span class="muted">· {t("status.every", { n: settings.value.interval_ms / 1000 })}</span>
    </div>
  </div>
</aside>

<style>
  .sidebar { width: 236px; flex: none; display: flex; flex-direction: column; background: var(--sidebar-tint); padding: 0 12px 12px; position: relative; }
  .drag { height: 52px; margin: 0 -12px; flex: none; }
  .mac { padding: 4px 10px 14px; }
  .name { font-weight: 650; font-size: 14px; letter-spacing: -0.01em; }
  .chip { color: var(--text-3); font-size: 12px; margin-top: 1px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  nav { display: flex; flex-direction: column; gap: 2px; overflow-y: auto; flex: 1; padding-bottom: 8px; }
  .group { font-size: 10.5px; font-weight: 650; letter-spacing: 0.07em; text-transform: uppercase; color: var(--text-3); padding: 12px 10px 5px; }
  .item { display: flex; align-items: center; gap: 10px; width: 100%; border: 0; background: transparent; padding: 7px 10px; border-radius: 9px; color: var(--text-2); cursor: pointer; text-align: left; font-weight: 520; transition: background 0.1s, color 0.1s; }
  .item:hover { background: color-mix(in srgb, var(--text) 7%, transparent); color: var(--text); }
  .item.on { background: color-mix(in srgb, var(--accent) 22%, transparent); color: var(--text); }
  .item.on :global(svg) { color: var(--accent); }
  .foot { display: flex; flex-direction: column; gap: 10px; padding-top: 8px; border-top: 1px solid var(--line); }
  .status { display: flex; align-items: center; gap: 7px; font-size: 11.5px; color: var(--text-2); padding: 0 10px; flex-wrap: wrap; }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--good); box-shadow: 0 0 0 3px color-mix(in srgb, var(--good) 22%, transparent); animation: pulse 2.4s ease-in-out infinite; }
  .dot.off { background: var(--text-3); box-shadow: none; animation: none; }
  @keyframes pulse { 50% { box-shadow: 0 0 0 5px color-mix(in srgb, var(--good) 8%, transparent); } }
</style>
