<script lang="ts">
  // Line icons, drawn on a 24px grid. Decorative unless a `label` is given.
  type Name =
    | "overview" | "cpu" | "memory" | "disk" | "network" | "gpu" | "sensors" | "battery" | "processes" | "history"
    | "alerts" | "settings" | "search" | "download" | "upload" | "check" | "chevron" | "fan" | "bolt" | "table" | "chart" | "trash" | "info";
  let { name, size = 18, label }: { name: Name; size?: number; label?: string } = $props();

  const paths: Record<Name, string> = {
    overview: "M4 5h7v7H4zM13 5h7v4h-7zM13 11h7v8h-7zM4 14h7v5H4z",
    cpu: "M8 8h8v8H8zM3 9h2M3 15h2M19 9h2M19 15h2M9 3v2M15 3v2M9 19v2M15 19v2",
    memory: "M3 8h18v8H3zM7 8v8M11 8v8M15 8v8M19 8v8M5 16v3M9 16v3M13 16v3M17 16v3",
    disk: "M4 7a8 3 0 0 1 16 0v10a8 3 0 0 1-16 0zM4 7a8 3 0 0 0 16 0M4 12a8 3 0 0 0 16 0",
    network: "M12 20a1.4 1.4 0 1 0 0-.01M8.6 16.2a5 5 0 0 1 6.8 0M5.6 13a9.2 9.2 0 0 1 12.8 0M2.8 9.8a13.4 13.4 0 0 1 18.4 0",
    gpu: "M3 7h15v10H3zM18 9h3v6h-3zM7 11.5a2.5 2.5 0 1 0 5 0 2.5 2.5 0 1 0-5 0",
    sensors: "M10 14.5V5a2 2 0 0 1 4 0v9.5a4 4 0 1 1-4 0zM12 9v7",
    battery: "M3 8h15v8H3zM18 11h2.5v2H18zM6 11v2",
    processes: "M4 6h16M4 12h16M4 18h10",
    history: "M4 12a8 8 0 1 0 2.4-5.7L4 8.5M4 4v4.5h4.5M12 8v4.5l3 1.8",
    alerts: "M6 16V11a6 6 0 1 1 12 0v5l1.5 2h-15zM10 21h4",
    settings: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 13.5l1.6 1.2-2 3.4-1.9-.6a7 7 0 0 1-1.6 1l-.4 2h-4l-.4-2a7 7 0 0 1-1.6-1l-1.9.6-2-3.4 1.6-1.2a7 7 0 0 1 0-2l-1.6-1.2 2-3.4 1.9.6a7 7 0 0 1 1.6-1l.4-2h4l.4 2a7 7 0 0 1 1.6 1l1.9-.6 2 3.4-1.6 1.2a7 7 0 0 1 0 2z",
    search: "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM21 21l-4.5-4.5",
    download: "M12 4v11M7 11l5 5 5-5M5 20h14",
    upload: "M12 20V9M7 13l5-5 5 5M5 4h14",
    check: "M5 12.5l4.5 4.5L19 7.5",
    chevron: "M9 6l6 6-6 6",
    fan: "M12 12c0-4 1-7 4-7 2.5 0 2 3-1 5M12 12c4 0 7 1 7 4 0 2.500-3 2-5-1M12 12c0 4-1 7-4 7-2.5 0-2-3 1-5M12 12c-4 0-7-1-7-4 0-2.5 3-2 5 1",
    bolt: "M13 3L5 13.5h6L10 21l8-10.500h-6z",
    table: "M4 5h16v14H4zM4 10h16M4 15h16M10 5v14",
    chart: "M4 19V5M4 19h16M7 15l4-5 3 3 5-7",
    trash: "M5 7h14M10 4h4M7 7l1 13h8l1-13M10 11v6M14 11v6",
    info: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 11v6M12 7.5v.01",
  };
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" role={label ? "img" : undefined} aria-label={label} aria-hidden={label ? undefined : true}>
  <path d={paths[name]} />
</svg>
