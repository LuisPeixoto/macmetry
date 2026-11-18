# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org).

## [Unreleased]

## [0.1.0] - 2026-11-18

The first release.

### Added

- **A native app for macOS 13 or newer, on Intel and Apple silicon**, built with Rust and Tauri, with a frosted-glass window,
  dark and light themes, and English and Brazilian Portuguese.
- **Readings** for the system, CPU (per core and per cluster, load average, thermal pressure), memory (app, wired,
  compressed, cached, swap, pressure, paging), disks (volumes, throughput per physical disk), network (per interface,
  with the interface type), GPU (usage, memory, clocks, and temperature, power and fan where the driver reports them),
  temperatures and fans from the SMC, power (Intel package energy counters and the Apple silicon energy model), the
  HID temperature sensors of Apple silicon, battery, and processes.
- **Views** for the overview, CPU, memory, disk, network, GPU, sensors, battery, processes, history, alerts and settings,
  with `Cmd+1` to `Cmd+9` to switch. A part a Mac does not have is hidden.
- **A menu bar item** with up to four live numbers and a popover, and the option to keep running when the window
  closes or to hide the Dock icon.
- **A history in SQLite**: one-second readings for 24 hours and one-minute average, lowest and highest for 30 days,
  charted with ranges from 1 hour to 30 days, and exported to **CSV and JSON**.
- **Alerts** with a duration and a cooldown, six suggested rules (two on by default), and system notifications.
- **A command line**, `macmetry`: `info`, `snapshot`, `watch`, `top`, `sensors`, `battery`, `history`, `export`,
  `record`, `metrics` and `mcp`.
- **An MCP server** with read-only tools, prompts and resources. Network addresses are never sent to the assistant.
- Opt-in live tests and scripts to capture fixtures on other Macs (`tools/live-tests.sh`, `tools/capture-fixtures.sh`).

### Known limits

- The Apple silicon collectors read the same interfaces as Apple's own tools and are tested with fixtures, but were
  developed on an Intel Mac and are **not yet confirmed on real Apple silicon hardware**.
- The two battery fixtures are synthetic, because the development Mac has no battery.
- Only the current user's processes are visible without administrator rights.
- The release builds are not yet signed or notarized.

[Unreleased]: https://github.com/LuisPeixoto/macmetry/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/LuisPeixoto/macmetry/releases/tag/v0.1.0
