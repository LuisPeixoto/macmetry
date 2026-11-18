<script lang="ts">
  import { onMount } from "svelte";
  import { live } from "./lib/stores/live.svelte.ts";
  import { settings } from "./lib/stores/settings.svelte.ts";
  import { nav } from "./lib/stores/nav.svelte.ts";
  import { availableViews } from "./lib/stores/nav.ts";
  import { t } from "./lib/stores/i18n.svelte.ts";
  import Sidebar from "./lib/components/layout/Sidebar.svelte";
  import Overview from "./lib/components/views/Overview.svelte";
  import Cpu from "./lib/components/views/Cpu.svelte";
  import Memory from "./lib/components/views/Memory.svelte";
  import Disk from "./lib/components/views/Disk.svelte";
  import Network from "./lib/components/views/Network.svelte";
  import Gpu from "./lib/components/views/Gpu.svelte";
  import Sensors from "./lib/components/views/Sensors.svelte";
  import Battery from "./lib/components/views/Battery.svelte";
  import EmptyState from "./lib/components/ui/EmptyState.svelte";

  onMount(() => {
    settings.load();
    live.start();
    // Cmd+1 to Cmd+9 jump to the views, in the order the sidebar lists them.
    const keys = (e: KeyboardEvent) => {
      if (!e.metaKey || e.altKey || e.ctrlKey) return;
      const n = Number(e.key);
      const items = availableViews(live.info?.capabilities ?? null);
      if (n >= 1 && n <= 9 && items[n - 1]) { e.preventDefault(); nav.go(items[n - 1].id); }
    };
    window.addEventListener("keydown", keys);
    return () => { window.removeEventListener("keydown", keys); live.dispose(); };
  });
</script>

<div class="app">
  <Sidebar />
  <main class="content">
    <div class="drag" data-tauri-drag-region></div>
    {#if !live.ready || !live.snapshot}
      <EmptyState icon="overview" title={t("status.starting")} />
    {:else if nav.view === "overview"}
      <Overview />
    {:else if nav.view === "cpu"}
      <Cpu />
    {:else if nav.view === "memory"}
      <Memory />
    {:else if nav.view === "disk"}
      <Disk />
    {:else if nav.view === "network"}
      <Network />
    {:else if nav.view === "gpu"}
      <Gpu />
    {:else if nav.view === "sensors"}
      <Sensors />
    {:else if nav.view === "battery"}
      <Battery />
    {:else}
      <EmptyState icon="info" title={t(`nav.${nav.view}`)} />
    {/if}
  </main>
</div>

<style>
  .app { display: flex; height: 100%; }
  .content { flex: 1; min-width: 0; display: flex; flex-direction: column; background: var(--panel); border-left: 1px solid var(--line); position: relative; }
  .drag { position: absolute; inset: 0 0 auto 0; height: 22px; z-index: 2; }
</style>
