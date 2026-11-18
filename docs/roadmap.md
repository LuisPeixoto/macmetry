# Roadmap

macmetry grows in phases. Each phase ends with a release. Items are roughly in priority order inside a phase. Ideas
and pull requests are welcome; open an issue to discuss anything large first.

macmetry is for macOS only, on Intel (x86_64) and Apple silicon (arm64). Every metric ships working on both
architectures where the hardware has it, and a card that a Mac cannot supply is hidden instead of showing zeros.

## ✅ Phase 1: Observe everything (v0.1)

**Collectors** for the system, **CPU** (per core and per cluster, load average, frequency on Apple silicon, thermal
pressure), **memory** (app, wired, compressed, cached, swap, pressure, paging), **disk** (volumes, throughput per
physical disk), **network** (per interface, with its type), **GPU** (usage, memory, clocks; temperature, power and fan
on AMD), **sensors** (temperatures and fans from the SMC, and the HID temperature sensors of Apple silicon),
**power** (Intel package energy counters and the Apple silicon energy model), **battery** and **processes**
(see [Metrics](metrics.md), [Sensors](sensors.md)) · three faces on one core: a **Tauri app** with a menu bar item and
popover, the **`macmetry` command line** ([CLI](cli.md)) and a **read-only MCP server** ([MCP](mcp.md)) ·
**history** in SQLite ([History](history.md)) · **alerts** with notifications ([Alerts](alerts.md)) · **CSV and JSON
export** · English and Brazilian Portuguese, dark and light · opt-in live tests and a fixture capture script for other
Macs.

Not done in this phase, and carried forward: confirming the Apple silicon collectors on real hardware, replacing the
synthetic battery fixtures with real captures, and signing and notarizing the builds.

## Phase 2: Confirmed on every Mac, and a home on the web (v0.2)

Fixtures and fixes from real Apple silicon Macs and from laptops, signed and notarized builds, and a website built from
the same Markdown as the repository, with a sitemap and a link checker.

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
