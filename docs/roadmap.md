# Roadmap

macmetry grows in phases. Each phase ends with a release. Items are roughly in priority order inside a phase. Ideas
and pull requests are welcome; open an issue to discuss anything large first.

macmetry is for macOS only, on Intel (x86_64) and Apple silicon (arm64). Every metric ships working on both
architectures where the hardware has it, and a card that a Mac cannot supply is hidden instead of showing zeros.

## Phase 1: Observe everything (v0.1)

**Collectors** for the system (model, chip, performance and efficiency cores, uptime), **CPU** (total, per core, per
cluster, load average, thermal pressure, frequency on Apple silicon), **memory** (app, wired, compressed, cached,
swap, pressure, paging), **disk** (volumes, throughput and operations per second), **network** (per interface
throughput, packets and errors), **GPU** (utilization and memory; frequency and power on Apple silicon),
**sensors** (temperatures, fans and power from the SMC, plus the HID temperature sensors on Apple silicon),
**energy** (CPU, GPU, Neural Engine and DRAM power on Apple silicon), **battery** (charge, health, cycles,
temperature, power) and **processes** (CPU, memory, threads and disk I/O).

**Three faces on one core**: a Tauri desktop app with a menu bar item and popover, the `macmetry` command line
(`info`, `snapshot`, `watch`, `top`, `sensors`, `battery`, `history`, `export`) and a read-only MCP server.

**History** in SQLite (one-second samples for 24 hours, one-minute aggregates for 30 days), **threshold alerts**
with system notifications, **CSV and JSON export**, and an interface in English and Brazilian Portuguese.

## Phase 2: A home on the web (v0.2)

A website built from the same Markdown as the repository, with a sitemap, a link checker and screenshots. Guides in
more languages.

## Phase 3: Deeper energy (v0.3)

Energy impact per process and per app, power over time with cost estimates, and a battery drain timeline.

## Phase 4: Storage health (v0.4)

SMART and NVMe health, wear, written bytes, and a large-files explorer.

## Phase 5: Network detail (v0.5)

Per-process network usage, Wi-Fi quality (signal, noise, channel, rate), latency and DNS probes the user turns on.

## Ideas

- Desktop widgets and a Lock Screen-style compact window.
- Shortcuts and AppleScript support.
- Export to Prometheus and OpenTelemetry for people who monitor a fleet of Macs.
- Per-core frequency graphs and cluster residency heat maps.
- Comparing two time ranges.
