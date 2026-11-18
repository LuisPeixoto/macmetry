<script lang="ts">
  import { onMount } from "svelte";
  import { live } from "../../stores/live.svelte.ts";
  import { settings } from "../../stores/settings.svelte.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { t } from "../../stores/i18n.svelte.ts";
  import { getBackend } from "../../api.ts";
  import type { HistoryStats, Settings } from "../../types.ts";
  import Card from "../ui/Card.svelte";
  import Toggle from "../ui/Toggle.svelte";
  import Select from "../ui/Select.svelte";
  import Chip from "../ui/Chip.svelte";
  import Button from "../ui/Button.svelte";
  import Icon from "../ui/Icon.svelte";
  import Segmented from "../ui/Segmented.svelte";
  import SettingRow from "../ui/SettingRow.svelte";
  import ViewHeader from "../layout/ViewHeader.svelte";

  const s = $derived(settings.value);
  let stats = $state.raw<HistoryStats | null>(null);
  let error = $state("");
  let confirming = $state(false);

  async function set(patch: Partial<Settings>) {
    try { await settings.update(patch); error = ""; }
    catch (e) { error = String(e); }
  }
  async function refreshStats() {
    try { stats = await (await getBackend()).historyStats(); } catch { stats = null; }
  }
  onMount(refreshStats);

  async function wipe() {
    if (!confirming) { confirming = true; return; }
    try { await (await getBackend()).clearHistory(); confirming = false; await refreshStats(); }
    catch (e) { error = String(e); confirming = false; }
  }

  const MENU_ITEMS = ["cpu", "memory", "temp", "power", "network", "gpu"];
  const toggleItem = (item: string) => {
    const has = s.menu_bar_items.includes(item);
    if (!has && s.menu_bar_items.length >= 4) return;
    set({ menu_bar_items: has ? s.menu_bar_items.filter((i) => i !== item) : [...s.menu_bar_items, item] });
  };
</script>

<ViewHeader title={t("nav.settings")} subtitle={t("settings.subtitle")} />

<div class="scroll">
  <div class="stack">
    {#if error}<p class="msg">{error}</p>{/if}

    <Card title={t("settings.appearance")} icon="settings">
      <SettingRow label={t("settings.language")} description={t("settings.language_text")}>
        <Select label={t("settings.language")} value={s.language} onchange={(v) => set({ language: v })}
          options={[{ value: "system", label: t("settings.system") }, { value: "en", label: "English" }, { value: "pt-BR", label: "Português (Brasil)" }]} />
      </SettingRow>
      <SettingRow label={t("settings.theme")}>
        <Segmented label={t("settings.theme")} value={s.theme} onchange={(v) => set({ theme: v })}
          options={[{ value: "system", label: t("settings.system") }, { value: "dark", label: t("settings.dark") }, { value: "light", label: t("settings.light") }]} />
      </SettingRow>
    </Card>

    <Card title={t("settings.sampling")} icon="history">
      <SettingRow label={t("settings.interval")} description={t("settings.interval_text")}>
        <Segmented label={t("settings.interval")} value={String(s.interval_ms)} onchange={(v) => set({ interval_ms: Number(v) })}
          options={[500, 1000, 2000, 5000].map((ms) => ({ value: String(ms), label: `${ms / 1000} s` }))} />
      </SettingRow>
      <SettingRow label={t("settings.record")} description={t("settings.record_text")}>
        <Toggle label={t("settings.record")} checked={s.record_history} onchange={(v) => set({ record_history: v })} />
      </SettingRow>
    </Card>

    <Card title={t("settings.menu_bar")} icon="overview">
      <SettingRow label={t("settings.menu_bar_show")} description={t("settings.menu_bar_text")}>
        <Toggle label={t("settings.menu_bar_show")} checked={s.menu_bar_enabled} onchange={(v) => set(v ? { menu_bar_enabled: true } : { menu_bar_enabled: false, hide_dock_icon: false })} />
      </SettingRow>
      <SettingRow label={t("settings.menu_bar_items")} description={t("settings.menu_bar_items_text")}>
        {#each MENU_ITEMS as item (item)}
          <Chip label={t(`settings.item.${item}`)} on={s.menu_bar_items.includes(item)} disabled={!s.menu_bar_enabled || (!s.menu_bar_items.includes(item) && s.menu_bar_items.length >= 4)} onclick={() => toggleItem(item)} />
        {/each}
      </SettingRow>
      <SettingRow label={t("settings.keep_running")} description={t("settings.keep_running_text")}>
        <Toggle label={t("settings.keep_running")} checked={s.keep_running} onchange={(v) => set({ keep_running: v })} />
      </SettingRow>
      <SettingRow label={t("settings.hide_dock")} description={t("settings.hide_dock_text")}>
        <Toggle label={t("settings.hide_dock")} checked={s.hide_dock_icon} disabled={!s.menu_bar_enabled} onchange={(v) => set({ hide_dock_icon: v })} />
      </SettingRow>
    </Card>

    <Card title={t("settings.data")} icon="disk">
      <SettingRow label={t("settings.folder")} description={t("settings.folder_text")}>
        <code>{live.info?.data_dir ?? ""}</code>
      </SettingRow>
      <SettingRow label={t("settings.history_size")} description={stats ? t("settings.history_rows", { n: fmt.num(stats.raw_rows + stats.minute_rows) }) : undefined}>
        <b class="num">{stats ? fmt.disk(stats.file_bytes) : "–"}</b>
      </SettingRow>
      <SettingRow label={t("settings.delete_history")} description={t("settings.delete_history_text")}>
        <Button kind="danger" onclick={wipe}><Icon name="trash" size={14} />{confirming ? t("settings.confirm_delete") : t("settings.delete")}</Button>
        {#if confirming}<Button onclick={() => (confirming = false)}>{t("alerts.cancel")}</Button>{/if}
      </SettingRow>
    </Card>

    <Card title={t("settings.about")} icon="info">
      <SettingRow label="macmetry" description={t("settings.about_text")}>
        <span class="muted num">v{live.info?.version ?? ""}</span>
      </SettingRow>
      <SettingRow label={t("settings.privacy")} description={t("settings.privacy_text")}><span></span></SettingRow>
    </Card>
  </div>
</div>

<style>
  .msg { margin: 0; padding: 10px 14px; border-radius: 10px; background: var(--card); border: 1px solid color-mix(in srgb, var(--bad) 40%, transparent); color: var(--bad); font-size: 12.5px; user-select: text; -webkit-user-select: text; }
  code { font: 12px var(--mono); background: var(--card-2); border: 1px solid var(--line); padding: 4px 8px; border-radius: 7px; user-select: text; -webkit-user-select: text; max-width: 340px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
