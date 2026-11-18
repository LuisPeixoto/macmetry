# Collector research

How macOS exposes performance metrics **without root**, on Intel and Apple silicon, and which source macmetry uses
for each metric. The evidence comes from inspecting Apple's own tools with REA (static Mach-O inspection) and from
reading a real Mac. Nothing here was taken from another project's source.

## What was inspected

| Target | How | Result |
|---|---|---|
| `/usr/bin/powermetrics` (universal, x86_64 + arm64e) | REA `inspect_macho` (imports) and `strings` on each slice | Which libraries and APIs Apple uses to read power, frequency and thermal data |
| This Mac: MacPro7,1, Intel Xeon W, AMD Radeon RX 6600 XT, macOS 26.2 | `sysctl`, `ioreg`, `pmset` | What an Intel desktop with a discrete GPU exposes |

REA's deep provider (Hopper) was not available (Spotlight indexing is off on the volume), so there is **no
decompilation** behind this document: only imports, exports and strings. Every claim below is either an observed
import or string, or something read live from a Mac. Apple silicon values must be confirmed on a Mac with an M-series
chip with `tools/capture-fixtures.sh`.

## What `powermetrics` imports

Observed with REA on the x86_64 slice (Evidence `ev_a1ab462dd221f03f76c7bb7025b3a408e2fee796915befce71a7a1ba5a54301a`):

- **`libIOReport`** (private): `IOReportCopyChannelsInGroup`, `IOReportCreateSubscription`, `IOReportCreateSamples`,
  `IOReportCreateSamplesDelta`, `IOReportIterate`, `IOReportChannelGetChannelName`, `IOReportChannelGetUnit`,
  `IOReportSimpleGetIntegerValue`, `IOReportStateGetResidency`, `IOReportStateGetNameForIndex`,
  `IOReportScaleValue`, `IOReportMergeChannels`, `IOReportPrune`. This is the source of energy, residency and
  frequency data.
- **IOKit**: `IOServiceMatching`, `IOServiceGetMatchingService(s)`, `IOServiceOpen`, `IOConnectCallStructMethod`,
  the IORegistry functions and `IOPMCopyPowerStateInfo`. The strings `AppleSMC`, `SMC sensors`, `SMC version`,
  `AppleSmartBattery` and `IOAccelTypes` show the SMC, battery and GPU accelerator are read through the registry.
- **libSystem**: `host_statistics`, `proc_pidinfo`, `sysctl`, `sysctlbyname`, `coalition_info_resource_usage`,
  `notify_register_check` and `notify_get_state` (thermal pressure, `kOSThermalNotificationPressureLevelName`),
  `task_policy_get`.
- **Privileged by design**: `geteuid` is imported, and `powermetrics` refuses to run without root. macmetry reads the
  same sources directly, from user space, so only what those sources allow without root is available.

Thermal pressure strings: `Nominal`, `Moderate`, `Heavy`, `Trapping`, `Sleeping`.

## Apple silicon: IOReport

From the arm64e slice's strings: the groups **`Energy Model`**, **`CPU Stats`** (subgroups `CPU Complex Performance
States` and `CPU Core Performance States`), **`GPU Stats`** (`GPU Performance States`), **`AMC Stats`** and
**`SoC Stats`**; the channels `ECPU`, `PCPU`, `EACC_CPU`, `PACC_CPU`, `CPU Energy`, `ECPU Energy`, `PCPU Energy`,
`GPU Energy`, `DRAM`, and `ANE0`-style channels. The tool prints per-core active, idle and down residency and
frequency, and `CPU`, `GPU`, `ANE` and combined power in milliwatts.

How macmetry uses it:

1. Subscribe once to `Energy Model` (power) and to the CPU and GPU performance state groups (frequency).
2. Take a sample each tick and call `IOReportCreateSamplesDelta` against the previous one.
3. **Energy**: the delta of an energy channel divided by the elapsed seconds, scaled by `IOReportChannelGetUnit`
   (millijoules, microjoules or nanojoules), gives watts.
4. **Frequency**: state residencies weighted by the frequency table of each state (read from the `pmgr` device in
   the IORegistry), as a fraction of the sampled time.

These symbols are private and can change between macOS versions, so they are resolved with `dlopen` and `dlsym`. If a
symbol or group is missing, only that card is hidden.

## Intel

- **SMC** (`AppleSMC` user client, selector 2, read bytes and read key info): temperatures, fans and power.
  `#KEY` gives the key count, `FNum` the number of fans, `F0Ac`/`F0Mn`/`F0Mx` current, minimum and maximum rpm, and
  `T…` keys are temperatures. Key sets differ per model: macmetry enumerates keys, picks known ones from a catalog and
  falls back to a prefix and type rule (`sp78`, `flt `). **Only the read selectors are ever called.**
- **CPU frequency**: not available without root on Intel; the nominal frequency comes from `sysctl hw.cpufrequency`.
- **Thermal pressure** works on both architectures through `notify`.

## Both architectures

| Metric | Source | Notes |
|---|---|---|
| CPU ticks per core | `host_processor_info(PROCESSOR_CPU_LOAD_INFO)` | Deltas of user, system, idle and nice ticks |
| Load average | `getloadavg` | |
| Memory | `host_statistics64(HOST_VM_INFO64)`, `vm.swapusage`, `kern.memorystatus_vm_pressure_level` | App memory = internal pages minus purgeable; compressed from `compressor_page_count` |
| Disk space | `getfsstat` | Only local, user-visible volumes |
| Disk throughput | IORegistry `IOBlockStorageDriver` → `Statistics` | Bytes and operations, as counters |
| Network | `sysctl` `NET_RT_IFLIST2` (`if_msghdr2`, 64-bit counters) | Counters are deltas per tick |
| Processes | `proc_listallpids`, `proc_pidinfo(PROC_PIDTASKINFO)`, `proc_pid_rusage` | Other users' processes may be partly hidden without root |
| Battery | IORegistry `AppleSmartBattery`, `IOPSCopyPowerSourcesInfo` | Not present on desktops |
| GPU | IORegistry `IOAccelerator` → `PerformanceStatistics` | See below |

## What this Mac (MacPro7,1) shows

- `IOAccelerator` for the AMD Radeon RX 6600 XT has a `PerformanceStatistics` dictionary with `Device Utilization %`,
  `GPU Activity(%)`, `Core Clock(MHz)`, `Memory Clock(MHz)`, `Temperature(C)`, `Total Power(W)`, `Fan Speed(RPM)`,
  `Fan Speed(%)`, `inUseVidMemoryBytes`, `vramFreeBytes` and `HWChannel …` counters. Discrete AMD GPUs therefore give
  temperature, power and fan without the SMC. A fixture is in
  `crates/macmetry-sys/tests/fixtures/intel-macpro7-1/ioaccelerator.plist`.
- There is no battery (`AppleSmartBattery` has no entry, `pmset -g batt` says "AC Power"), so the battery card is
  hidden. This is why every collector must report "unsupported" instead of failing.
- `AppleSMC` exists in the registry.

## Not verified yet

- Apple silicon channel names, units and SMC/HID sensor names must be checked on an M-series Mac. The capture script
  `tools/capture-fixtures.sh` writes them to fixtures.
- Whether macOS 26 hides any IOHID temperature sensors from non-root clients on Apple silicon.
- Activity Monitor was not inspected.
