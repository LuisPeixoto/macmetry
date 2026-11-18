// Sorting and filtering for the processes table. Pure, so the order and the search are tested.

import type { ProcessSample } from "./types.ts";

export type SortKey = "name" | "pid" | "user" | "cpu" | "memory" | "threads" | "read" | "write";

const NUMERIC: Record<Exclude<SortKey, "name" | "user">, (p: ProcessSample) => number> = {
  pid: (p) => p.pid,
  cpu: (p) => p.cpu_pct,
  memory: (p) => p.memory_bytes,
  threads: (p) => p.threads,
  read: (p) => p.disk_read_bps,
  write: (p) => p.disk_write_bps,
};

/** Text columns start at A and number columns start at the biggest, which is what a person sorting by one expects. */
export const defaultDirection = (key: SortKey): "asc" | "desc" => (key === "name" || key === "user" ? "asc" : "desc");

/** A sorted copy. Ties are broken by pid, so rows do not shuffle when two have the same value. */
export function sortProcesses(list: readonly ProcessSample[], key: SortKey, direction: "asc" | "desc"): ProcessSample[] {
  const sign = direction === "asc" ? 1 : -1;
  const compare = (a: ProcessSample, b: ProcessSample): number => {
    if (key === "name") return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
    if (key === "user") return a.user.localeCompare(b.user, undefined, { sensitivity: "base" });
    return NUMERIC[key](a) - NUMERIC[key](b);
  };
  return [...list].sort((a, b) => sign * compare(a, b) || a.pid - b.pid);
}

/** Processes whose name, user or pid contains the text, ignoring case. An empty search keeps everything. */
export function filterProcesses(list: readonly ProcessSample[], search: string): ProcessSample[] {
  const needle = search.trim().toLowerCase();
  if (!needle) return [...list];
  return list.filter((p) => p.name.toLowerCase().includes(needle) || p.user.toLowerCase().includes(needle) || String(p.pid) === needle);
}
