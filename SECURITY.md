# Security policy

macmetry reads system metrics, process information and sensor values from your Mac. It does not send them anywhere.

## Supported versions

Only the latest release receives security fixes while the project is below 1.0.

## Reporting a vulnerability

**Please do not open a public issue.** Use GitHub's private reporting:

<https://github.com/LuisPeixoto/macmetry/security/advisories/new>

Include, where possible, a description and the impact, steps to reproduce, and the affected versions and macOS
versions. You will get an acknowledgement as soon as the maintainer can review it.

## What is in scope

- Anything that writes to hardware or system state (macmetry only reads sensors)
- Network access the app was not documented to have (it should make none)
- Data written outside the app's own data folder, or without an explicit user action
- Crashes or unbounded memory use caused by hostile input in parsers and in the MCP server

## Design notes

What macmetry does with your data and your Mac, feature by feature.

**Reading the system.** All readings come from system interfaces any program can use without administrator rights:
kernel counters, the I/O Registry, the SMC and two private frameworks (IOReport and the HID event system). The private
ones are loaded with `dlopen` at run time, so a macOS update that changes them only hides the cards that depend on
them. macmetry never asks for a password and never installs a helper.

**The SMC is read-only.** Only three commands are ever sent to the System Management Controller: read a key, read a
key's type, and list keys. The write command is not in the code (`crates/macmetry-sys/src/smc.rs`), so macmetry cannot
change a fan speed or anything else.

**Foreign function calls.** Raw system calls are confined to a few modules (`sysctl`, `cf`, `iokit`, `smc`, `ioreport`,
`hid`, and a few more) and every `unsafe` block carries a `SAFETY` comment, enforced by Clippy. CoreFoundation trees are
converted with depth and size limits, and registry and routing-socket parsers skip anything truncated or impossible
instead of trusting it.

**Network.** macmetry makes **no network connections**. The desktop app's content security policy allows only the local
IPC channel. There is no update check, no analytics and no crash reporting.

**What is stored.** A history database (numbers and timestamps only: no process names, no network addresses) and a
`settings.json`, both in `~/Library/Application Support/macmetry`, or in `$MACMETRY_HOME`. The settings file is written
to a temporary file and renamed, so a crash cannot leave half a file, and a damaged file falls back to the defaults.

**Files macmetry writes.** Only the two above, and an export **after you choose where to save it**. An export from the
command line never replaces an existing file unless `--force` is given.

**The command line and the MCP server** open the history read-only. The MCP server offers only read-only tools, marks
each as such (a test fails otherwise), refuses anything that could name a file path, **removes network addresses** from
every snapshot before an assistant sees it, and bounds everything: watches (4 at once, a size cap, and a stop after 5
minutes unread or 1 hour), history points, and the size of a reply. Prompt arguments that end up inside a plan are
validated so free text cannot be slipped in as an instruction.

**Alerts and notifications** are local. The alert engine reads your own measurements and decides; nothing leaves the Mac.

**Process names** can be personal. They are shown in the app and the command line, and they are included in what the MCP
server sends to an assistant you connect, which is why `snapshot` lets you ask for sections without processes.

**Unsigned builds.** Until releases are signed and notarized, verify a download against `SHA256SUMS`.
