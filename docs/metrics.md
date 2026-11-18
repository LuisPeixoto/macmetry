# Metrics

Every number macmetry shows is one of these, and each has a stable key (for example `cpu.total`) that the history, the
alerts, the command line and the MCP server all share. `macmetry metrics` lists them.

A part your Mac does not have is **absent, never zero**. A desktop has no battery, an Intel Mac has no efficiency
cores, and a Mac that does not share its temperatures shows no temperature card.

## CPU

| Key | What it is |
|---|---|
| `cpu.total` | Percent of all cores that was not idle, over the last reading |
| `cpu.user`, `cpu.system` | The user and system share of that time |
| `cpu.load1` | Load average over one minute |

Per-core bars come from the kernel's tick counters. On Apple silicon the cores are split into efficiency and
performance, with a cluster usage and frequency. On Intel macOS shows only the **nominal** frequency without
administrator rights, and macmetry says so.

## Memory

| Key | What it is |
|---|---|
| `memory.used_pct`, `memory.used` | App memory + wired + compressed, as Activity Monitor's "Memory Used" |
| `memory.pressure` | 0 normal, 1 warning, 2 critical |
| `memory.swap_used` | Swap in use |
| `memory.page_outs` | Pages written to disk per second |

**Cached files** are memory macOS keeps only because there is room, and gives back at once when an app needs it, so a
full bar is not a problem. Watch the **pressure** instead.

## Disk and network

| Key | What it is |
|---|---|
| `disk.read_bps`, `disk.write_bps` | Throughput summed over the physical disks |
| `disk.free_pct` | Free space on the startup volume |
| `net.rx_bps`, `net.tx_bps` | Download and upload, summed over the interfaces that are not loopback |

Disk sizes use 1000s, as Finder does, and memory uses 1024s, as Activity Monitor does. Network addresses appear in the
window and the command line, and are **never** sent to an AI assistant.

## GPU, sensors and power

| Key | What it is |
|---|---|
| `gpu.utilization`, `gpu.temperature` | The busiest GPU's usage and temperature, where the driver reports them |
| `temp.cpu`, `temp.max` | The hottest CPU sensor, and the hottest sensor of all |
| `fan.rpm` | The fastest fan |
| `power.cpu`, `power.gpu`, `power.system` | Watts, where the Mac reports them |
| `thermal.pressure` | 0 nominal, 1 moderate, 2 heavy, 3 throttling, 4 cooling down |

See [Sensors, power and what each Mac can report](sensors.md) for where each one comes from.

## Battery

`battery.percent` and `battery.temperature`. The window also shows health (maximum capacity against design), cycles,
power in watts, time remaining and the charger.

## Processes

CPU is a percentage of **one core**, so 200% means two cores. Memory is the physical footprint, the figure Activity
Monitor shows. Only the current user's processes are visible without administrator rights.
