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
