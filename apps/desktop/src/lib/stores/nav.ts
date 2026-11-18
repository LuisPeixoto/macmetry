import type { Capabilities } from "../types.ts";

export type ViewId = "overview" | "cpu" | "memory" | "disk" | "network" | "gpu" | "sensors" | "battery" | "processes" | "history" | "alerts" | "settings";

export interface NavItem { id: ViewId; icon: string; group: "monitor" | "data" | "app" }

export const NAV: NavItem[] = [
  { id: "overview", icon: "overview", group: "monitor" },
  { id: "cpu", icon: "cpu", group: "monitor" },
  { id: "memory", icon: "memory", group: "monitor" },
  { id: "disk", icon: "disk", group: "monitor" },
  { id: "network", icon: "network", group: "monitor" },
  { id: "gpu", icon: "gpu", group: "monitor" },
  { id: "sensors", icon: "sensors", group: "monitor" },
  { id: "battery", icon: "battery", group: "monitor" },
  { id: "processes", icon: "processes", group: "monitor" },
  { id: "history", icon: "history", group: "data" },
  { id: "alerts", icon: "alerts", group: "data" },
  { id: "settings", icon: "settings", group: "app" },
];

/** The views this Mac can show: no battery view on a desktop, no sensors page on a machine that has none. */
export function availableViews(caps: Capabilities | null): NavItem[] {
  if (!caps) return NAV;
  return NAV.filter((item) => {
    if (item.id === "battery") return caps.battery;
    if (item.id === "gpu") return caps.gpu;
    if (item.id === "sensors") return caps.temperatures || caps.fans || caps.power;
    return true;
  });
}
