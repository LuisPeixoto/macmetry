import { test } from "node:test";
import assert from "node:assert/strict";
import { defaultDirection, filterProcesses, sortProcesses } from "./processes.ts";
import type { ProcessSample } from "./types.ts";

const p = (pid: number, name: string, over: Partial<ProcessSample> = {}): ProcessSample => ({
  pid, name, user: "luis", cpu_pct: 0, memory_bytes: 0, threads: 1, disk_read_bps: 0, disk_write_bps: 0, ...over,
});
const list = [p(3, "Safari", { cpu_pct: 5, memory_bytes: 900 }), p(1, "xcode", { cpu_pct: 40, memory_bytes: 300 }), p(2, "Mail", { cpu_pct: 5, memory_bytes: 100, user: "_root" })];
const pids = (l: ProcessSample[]) => l.map((x) => x.pid);

test("numbers sort biggest first by default and text sorts A to Z", () => {
  assert.equal(defaultDirection("cpu"), "desc");
  assert.equal(defaultDirection("name"), "asc");
  assert.deepEqual(pids(sortProcesses(list, "cpu", "desc")), [1, 2, 3], "a tie in CPU is broken by pid");
  assert.deepEqual(pids(sortProcesses(list, "memory", "desc")), [3, 1, 2]);
  assert.deepEqual(pids(sortProcesses(list, "memory", "asc")), [2, 1, 3]);
});

test("names sort without regard to case", () => {
  assert.deepEqual(sortProcesses(list, "name", "asc").map((x) => x.name), ["Mail", "Safari", "xcode"]);
  assert.deepEqual(sortProcesses(list, "user", "asc").map((x) => x.user), ["_root", "luis", "luis"]);
});

test("sorting does not change the list it was given", () => {
  const before = pids(list);
  sortProcesses(list, "cpu", "desc");
  assert.deepEqual(pids(list), before);
});

test("the search matches names, users and an exact pid, ignoring case", () => {
  assert.deepEqual(pids(filterProcesses(list, "SAF")), [3]);
  assert.deepEqual(pids(filterProcesses(list, "_root")), [2]);
  assert.deepEqual(pids(filterProcesses(list, "1")), [1], "a pid must match exactly, so 1 is not 10 or 21");
  assert.equal(filterProcesses(list, "   ").length, 3, "a blank search keeps everything");
  assert.equal(filterProcesses(list, "nothing here").length, 0);
});
