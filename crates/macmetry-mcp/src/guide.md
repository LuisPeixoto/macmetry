# macmetry for assistants

macmetry reads the performance of the Mac it runs on. **Every tool only reads; nothing here changes the Mac.**

## Which tool answers which question

| Question | Tool |
|---|---|
| What Mac is this, and what can it report? | `system_info` |
| How is it doing right now? | `snapshot` (use `sections` to ask for only what you need) |
| What is using the CPU, memory or disk? | `processes` |
| Is it hot? Are the fans running? Is it throttling? | `sensors` |
| How is the battery? | `battery` |
| What happened over the last hour or day? | `history_query` |
| What changes while I do something? | `watch_start`, `watch_read`, `watch_stop` |
| Which metrics exist? | `list_metrics` |

## Reading the numbers

- A **process's CPU** is a percentage of one core: 200 means two cores.
- **Memory pressure** is `normal`, `warning` or `critical`. Pressure matters more than how full memory looks, because
  macOS uses spare memory for caches on purpose.
- **Thermal pressure** above `nominal` means macOS is already slowing the Mac down to cool it.
- A history **point** has an average, a lowest and a highest value for the time it covers. The highest is what shows a
  short spike.
- A part a Mac does not have is **left out**, not shown as zero. A desktop has no battery; an Intel Mac has no
  efficiency cores and no Neural Engine reading.

## Limits

- Only the current user's processes are visible without administrator rights.
- The history keeps one-second samples for 24 hours and one-minute averages for 30 days, and only exists if the app or
  `macmetry record` was running.
- Network **addresses are never sent**; speeds and totals are.
- At most 4 watches run at once, and a watch nobody reads for 5 minutes stops by itself.
