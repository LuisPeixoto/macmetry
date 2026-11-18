# Sensors, power and what each Mac can report

macmetry reads sensors **without root**. That is more limited than Apple's own `powermetrics` tool, which needs
`sudo`, and the limits differ between Intel and Apple silicon. This page says what to expect.

## Where the readings come from

| Reading | Intel | Apple silicon |
|---|---|---|
| CPU, memory, disk, network, processes | kernel counters | kernel counters |
| Temperatures | SMC keys (`TC0P`, `TC0C`...) | SMC and HID temperature sensors |
| Fans | SMC | SMC |
| CPU power | package energy counters (RAPL), through IOReport | the `Energy Model` through IOReport |
| GPU, ANE and memory power | the GPU driver, where it reports power | the `Energy Model` |
| CPU frequency | nominal only | measured, from the performance-state residency |
| GPU | the driver's statistics (utilization, memory, clocks; temperature, power and fan on AMD) | the driver's statistics |
| Battery | `AppleSmartBattery` | `AppleSmartBattery` |
| Thermal pressure | the system notification | the system notification |

IOReport and the HID sensors are **private** frameworks. macmetry looks them up at run time instead of linking to them,
so if a macOS update changes one, only the cards that depend on it disappear. See [Security](../SECURITY.md).

## Why a card may be missing

- **No battery.** A desktop Mac has none, so the Battery view is hidden.
- **No temperature sensors.** A virtual machine, or a Mac that does not expose them, has none.
- **No GPU statistics.** Only drivers that publish performance statistics appear.
- **No frequency on Intel.** macOS gives the real frequency only to administrators.

If you think a Mac should report something it does not, run `macmetry snapshot --json` and open an issue with the
output, after removing anything you do not want to share. `tools/capture-fixtures.sh` captures more detail.

## The SMC is only read

macmetry sends three commands to the System Management Controller: read a key, read a key's type and list the keys.
The write command is **not in the code** (`crates/macmetry-sys/src/smc.rs` defines only the three read commands), so
macmetry cannot change a fan speed or any other setting. Anyone can check that in one file.

## What was verified where

The collectors were developed on an Intel Mac Pro (MacPro7,1) with an AMD GPU. The Apple silicon collectors read the
same system interfaces that Apple's tools use, and their parsers are tested with fixtures, but they must be confirmed
on real Apple silicon hardware. `tools/live-tests.sh` and `tools/capture-fixtures.sh` exist for that, and every report
helps.
