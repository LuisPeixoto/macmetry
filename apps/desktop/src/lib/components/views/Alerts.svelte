<script lang="ts">
  import { live } from "../../stores/live.svelte.ts";
  import { settings } from "../../stores/settings.svelte.ts";
  import { fmt } from "../../stores/fmt.svelte.ts";
  import { t } from "../../stores/i18n.svelte.ts";
  import { METRICS } from "../../metrics.ts";
  import { COOLDOWN_OPTIONS, displayName, durationLabel, FOR_OPTIONS, fieldFor, fromInput, levelToThreshold, newRule, problem, thresholdToLevel, toInput } from "../../alerts.ts";
  import { defaultSettings } from "../../defaults.ts";
  import type { Rule } from "../../types.ts";
  import Card from "../ui/Card.svelte";
  import Toggle from "../ui/Toggle.svelte";
  import Select from "../ui/Select.svelte";
  import Button from "../ui/Button.svelte";
  import Icon from "../ui/Icon.svelte";
  import SettingRow from "../ui/SettingRow.svelte";
  import ViewHeader from "../layout/ViewHeader.svelte";

  const rules = $derived(settings.value.alerts);
  const suggested = defaultSettings().alerts;
  const nameOf = (r: Rule) => displayName(r, suggested, (id) => t(`alerts.preset.${id}`));
  let editing = $state<string | null>(null);
  let adding = $state("cpu.total");
  let error = $state("");

  const metricOf = (key: string) => METRICS.find((m) => m.key === key)!;
  // Offer a metric only when this Mac has produced a value for it, so nobody sets a battery alert on a desktop.
  const choices = $derived(METRICS.filter((m) => (live.series[m.key] ?? []).some((v) => v !== null) || rules.some((r) => r.metric === m.key)));

  async function save(next: Rule[]) {
    try { await settings.update({ alerts: next }); error = ""; }
    catch (e) { error = String(e); }
  }
  const change = (id: string, patch: Partial<Rule>) => save(rules.map((r) => (r.id === id ? { ...r, ...patch } : r)));
  const remove = (id: string) => { if (editing === id) editing = null; return save(rules.filter((r) => r.id !== id)); };
  async function add() {
    const rule = newRule(metricOf(adding), rules);
    await save([...rules, rule]);
    if (!error) editing = rule.id;
  }
  const restore = () => save(defaultSettings().alerts);

  const LEVELS: Record<string, { value: number; name: string }[]> = {
    "memory.pressure": [{ value: 1, name: "pressure.warning" }, { value: 2, name: "pressure.critical" }],
    "thermal.pressure": [{ value: 1, name: "thermal.moderate" }, { value: 2, name: "thermal.heavy" }, { value: 3, name: "thermal.trapping" }],
  };
  const levelName = (key: string, level: number) => t(LEVELS[key]?.find((l) => l.value === level)?.name ?? "common.none");

  function describe(r: Rule): string {
    const m = metricOf(r.metric);
    const f = fieldFor(m);
    const name = t(`metric.${r.metric}`);
    const how = f.kind === "level"
      ? t("alerts.when_level", { metric: name, level: levelName(r.metric, thresholdToLevel(r.threshold)) })
      : t("alerts.when", { metric: name, cmp: t(`alerts.${r.comparator}`), value: `${fmt.num(toInput(r.threshold, f), 1)} ${f.suffix}`.trim() });
    return r.for_secs ? `${how} · ${t("alerts.for", { time: durationLabel(r.for_secs) })}` : how;
  }
</script>

<ViewHeader title={t("nav.alerts")} subtitle={t("alerts.subtitle")} />

<div class="scroll">
  <div class="stack">
    {#if error}<p class="msg">{error}</p>{/if}

    <Card>
      <SettingRow label={t("alerts.notifications")} description={t("alerts.notifications_text")}>
        <Toggle label={t("alerts.notifications")} checked={settings.value.notifications} onchange={(v) => settings.update({ notifications: v })} />
      </SettingRow>
    </Card>

    <Card title={t("alerts.rules")} icon="alerts">
      {#snippet actions()}
        <Select label={t("alerts.metric")} value={adding} onchange={(v) => (adding = v)} options={choices.map((m) => ({ value: m.key, label: t(`metric.${m.key}`) }))} />
        <Button kind="primary" onclick={add}>{t("alerts.add")}</Button>
      {/snippet}

      <ul class="rules">
        {#each rules as r (r.id)}
          {@const m = metricOf(r.metric)}
          {@const f = fieldFor(m)}
          <li class:off={!r.enabled}>
            <div class="head">
              <Toggle label={nameOf(r)} checked={r.enabled} onchange={(v) => change(r.id, { enabled: v })} />
              <div class="what">
                <div class="name">{nameOf(r)}</div>
                <div class="muted desc">{describe(r)}</div>
              </div>
              <Button onclick={() => (editing = editing === r.id ? null : r.id)}>{editing === r.id ? t("alerts.done") : t("alerts.edit")}</Button>
            </div>

            {#if editing === r.id}
              <div class="edit">
                <label><span class="eyebrow">{t("alerts.name")}</span>
                  <input type="text" value={nameOf(r)} maxlength="80" onchange={(e) => change(r.id, { name: (e.currentTarget as HTMLInputElement).value })} aria-invalid={problem(r) === "name"} />
                </label>
                <label><span class="eyebrow">{t("alerts.metric")}</span>
                  <Select label={t("alerts.metric")} value={r.metric} onchange={(v) => { const nf = fieldFor(metricOf(v)); change(r.id, { metric: v, comparator: nf.kind === "level" ? "above" : r.comparator, threshold: newRule(metricOf(v), []).threshold }); }}
                    options={choices.map((x) => ({ value: x.key, label: t(`metric.${x.key}`) }))} />
                </label>
                {#if f.kind === "level"}
                  <label><span class="eyebrow">{t("alerts.reaches")}</span>
                    <Select label={t("alerts.reaches")} value={thresholdToLevel(r.threshold)} onchange={(v) => change(r.id, { comparator: "above", threshold: levelToThreshold(v) })}
                      options={(LEVELS[r.metric] ?? []).map((l) => ({ value: l.value, label: t(l.name) }))} />
                  </label>
                {:else}
                  <label><span class="eyebrow">{t("alerts.condition")}</span>
                    <span class="inline">
                      <Select label={t("alerts.condition")} value={r.comparator} onchange={(v) => change(r.id, { comparator: v })} options={[{ value: "above", label: t("alerts.above") }, { value: "below", label: t("alerts.below") }]} />
                      <input type="number" class="num" step={f.step} value={toInput(r.threshold, f)} onchange={(e) => change(r.id, { threshold: fromInput(Number((e.currentTarget as HTMLInputElement).value), f) })} aria-label={t("alerts.threshold")} />
                      <span class="muted">{f.suffix}</span>
                    </span>
                  </label>
                {/if}
                <label><span class="eyebrow">{t("alerts.for_label")}</span>
                  <Select label={t("alerts.for_label")} value={r.for_secs} onchange={(v) => change(r.id, { for_secs: v })} options={FOR_OPTIONS.map((s) => ({ value: s, label: durationLabel(s) }))} />
                </label>
                <label><span class="eyebrow">{t("alerts.cooldown")}</span>
                  <Select label={t("alerts.cooldown")} value={r.cooldown_secs} onchange={(v) => change(r.id, { cooldown_secs: v })} options={COOLDOWN_OPTIONS.map((s) => ({ value: s, label: durationLabel(s) }))} />
                </label>
                <div class="del"><Button kind="danger" onclick={() => remove(r.id)}><Icon name="trash" size={14} />{t("alerts.delete")}</Button></div>
              </div>
            {/if}
          </li>
        {/each}
      </ul>
      {#if !rules.length}<p class="muted">{t("alerts.none")}</p>{/if}
      <div class="foot"><Button onclick={restore}>{t("alerts.restore")}</Button></div>
    </Card>

    <Card title={t("alerts.recent")} icon="alerts">
      {#if live.alerts.length}
        <ul class="events">
          {#each live.alerts as a (a.rule_id + a.at_ms)}
            <li>
              <span class="pill {a.kind === 'fired' ? 'bad' : 'good'}">{a.kind === "fired" ? t("alerts.fired") : t("alerts.resolved")}</span>
              <span class="ev">{nameOf(rules.find((r) => r.id === a.rule_id) ?? { ...suggested[0], id: a.rule_id, name: a.name })}</span><span class="muted">{a.message}</span><span class="muted num t">{fmt.clock(a.at_ms, true)}</span>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="muted none">{t("alerts.quiet")}</p>
      {/if}
    </Card>
  </div>
</div>

<style>
  .msg { margin: 0; padding: 10px 14px; border-radius: 10px; background: var(--card); border: 1px solid color-mix(in srgb, var(--bad) 40%, transparent); color: var(--bad); font-size: 12.5px; user-select: text; -webkit-user-select: text; }
  .rules, .events { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
  .rules li { padding: 12px 0; border-top: 1px solid var(--line); }
  .rules li:first-child { border-top: 0; padding-top: 0; }
  .rules li.off .what { opacity: 0.55; }
  .head { display: flex; align-items: center; gap: 14px; }
  .what { flex: 1; min-width: 0; }
  .name { font-weight: 560; }
  .desc { font-size: 12.5px; }
  .edit { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 14px 18px; margin: 14px 0 4px 52px; padding: 14px; border-radius: 12px; background: var(--card-2); border: 1px solid var(--line); }
  label { display: flex; flex-direction: column; gap: 6px; }
  .inline { display: flex; align-items: center; gap: 8px; }
  input { height: 30px; padding: 0 10px; border-radius: 9px; border: 1px solid var(--line-2); background: var(--card); color: var(--text); font: inherit; width: 100%; user-select: text; -webkit-user-select: text; }
  input[type="number"] { width: 90px; }
  input[aria-invalid="true"] { border-color: var(--bad); }
  .del { display: flex; align-items: flex-end; }
  .foot { margin-top: 14px; padding-top: 14px; border-top: 1px solid var(--line); display: flex; justify-content: flex-end; }
  .events li { display: grid; grid-template-columns: 90px minmax(120px, 1fr) minmax(120px, 2fr) 70px; gap: 12px; align-items: center; padding: 9px 0; border-top: 1px solid var(--line); }
  .events li:first-child { border-top: 0; }
  .ev { font-weight: 560; }
  .t { text-align: right; }
  .none { margin: 0; }
</style>
