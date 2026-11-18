# MCP tools

<!-- Generated from the server by `UPDATE_MCP_DOCS=1 cargo test -p macmetry-mcp --test docs`. Do not edit by hand. -->

Every tool only reads. Start the server with `macmetry mcp`; see [the MCP guide](mcp.md).

## `battery`: Battery

The battery: charge, whether it is charging, health (maximum capacity against design), cycle count, temperature and power. A Mac without a battery says so.

No arguments.

## `history_query`: History of a metric

How one metric changed over the last minutes, hours or days, from the history macmetry recorded (the app, or `macmetry record`, must have been running). Each point has the average, lowest and highest value of the time it covers. The history keeps 24 hours at one second and 30 days at one minute.

| Argument | Type | Required | What it is |
|---|---|---|---|
| `metric` | string | yes | The metric's key, for example cpu.total or temp.max. list_metrics shows them all. |
| `points` | integer | no | How many points to return at most (2 to 500, default 60). Each point has the average, lowest and highest value of the time it covers, so a short spike is not lost. |
| `since` | string | no | How far back to look: 30s, 15m, 6h or 7d (default 1h; the history keeps 30 days). |

## `list_metrics`: List metrics

Every metric the history and the watches understand: its key, name and unit.

No arguments.

## `processes`: Busiest processes

The busiest processes, sorted by CPU, memory or disk use, with their CPU percentage (of one core, so 200 means two cores), memory footprint, threads, user and disk throughput. Only processes of the current user are visible without administrator rights.

| Argument | Type | Required | What it is |
|---|---|---|---|
| `filter` | string | no | Only processes whose name contains this text (case-insensitive). |
| `limit` | integer | no | How many processes to return (1 to 100, default 15). |
| `sort` | string | no | What to sort by: cpu, memory, disk_read, disk_write or name (default cpu). |

## `sensors`: Temperatures, fans and power

Temperatures (CPU cores, GPU, storage, battery...), fan speeds, power draw by component and thermal pressure. A hot Mac that is not slowing down has `nominal` pressure; `moderate`, `heavy` and `trapping` mean macOS is reducing speed to cool off.

No arguments.

## `snapshot`: Measure now

Measure everything now: CPU (total, per core), memory (used, wired, compressed, swap, pressure), disks (space and throughput), network (speed per interface, no addresses), GPU, temperatures and fans, power, battery, thermal pressure and the busiest processes. Parts this Mac does not have are left out. Use `sections` to ask for only what you need.

| Argument | Type | Required | What it is |
|---|---|---|---|
| `processes` | integer | no | How many of the busiest processes to include (0 to 50, default 5). |
| `sections` | list of string | no | Only these parts: any of cpu, memory, disks, network, gpus, sensors, power, battery, thermal, processes. Leave out for everything. Asking for fewer parts saves tokens. |
| `wait_ms` | integer | no | How long to measure before answering, in milliseconds (200 to 5000, default 1000). Rates such as CPU use and network speed need a little time to mean anything. |

## `system_info`: About this Mac

Describe this Mac: model, chip, architecture (Intel or Apple silicon), cores, memory and macOS version, and which kinds of readings it can provide (a desktop has no battery; Intel has no efficiency cores). Start here.

No arguments.

## `watch_read`: Read a watch

Read what a watch collected. Pass the previous call's `next_since_ms` as `since_ms` to get only the new readings.

| Argument | Type | Required | What it is |
|---|---|---|---|
| `id` | string | yes | The id watch_start returned. |
| `since_ms` | integer | no | Only readings after this time, in milliseconds since the epoch (the `next_since` of the previous read). Leave out for everything kept. |

## `watch_start`: Start a watch

Start following a few metrics live, for example while you ask the person to open an app. Readings are kept in a short buffer; call watch_read to get them and watch_stop when done. At most 4 watches run at once, and one nobody reads for 5 minutes stops by itself.

| Argument | Type | Required | What it is |
|---|---|---|---|
| `interval_secs` | number | no | Seconds between readings (1 to 60, default 2). |
| `max_points` | integer | no | How many readings to keep (10 to 600, default 300). Older ones are dropped. |
| `metrics` | list of string | yes | The metrics to follow (1 to 10 keys, see list_metrics). |

## `watch_stop`: Stop a watch

Stop a watch started with watch_start and free it. Call it when you have the readings you need; a watch left running keeps measuring until it goes unread for 5 minutes.

| Argument | Type | Required | What it is |
|---|---|---|---|
| `id` | string | yes | The id watch_start returned. |

