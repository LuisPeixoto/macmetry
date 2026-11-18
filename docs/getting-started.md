# Getting started

macmetry runs on macOS 13 (Ventura) or newer, on Intel and Apple silicon.

## Install

Download the `.dmg` for your Mac from the [latest release](https://github.com/LuisPeixoto/macmetry/releases/latest):
`aarch64` is Apple silicon, `x64` is Intel. Open it and drag macmetry to Applications.

The builds are not signed with an Apple Developer ID yet (see [Distribution](distribution.md)), so macOS asks you to
confirm the first time. Either right-click the app and choose **Open**, or remove the quarantine flag once:

```bash
xattr -dr com.apple.quarantine /Applications/macmetry.app
```

The command line is a separate download, `macmetry-<version>-<target>.tar.gz`, which holds one binary called `macmetry`.
Check any download against `SHA256SUMS` on the release page.

## First run

macmetry opens a window and puts an item in the menu bar. Nothing needs permission: it reads what macOS shares with any
program, and it never asks for your password.

- The **window** has a sidebar with a view for each part of your Mac. `Cmd+1` to `Cmd+9` jump between them.
- The **menu bar item** shows live numbers. Click it for a quick look, with a button to open the window.
- Closing the window does not quit macmetry: it keeps recording and watching your alerts from the menu bar. Quit it
  from the menu bar item, or turn this off in **Settings**.

## What you can do next

- Understand each number: [Metrics](metrics.md).
- See why a card is missing on your Mac: [Sensors, power and what each Mac can report](sensors.md).
- Look back in time and export: [History](history.md).
- Get notified: [Alerts](alerts.md).
- Use it from a terminal: [Command line](cli.md), or from an AI assistant: [MCP server](mcp.md).

## Build it yourself

You need [Rust](https://rustup.rs) (stable) and [Node.js](https://nodejs.org) 20 or newer.

```bash
git clone https://github.com/LuisPeixoto/macmetry.git
cd macmetry
cd apps/desktop && npm install && npm run tauri dev
```

See [Contributing](../CONTRIBUTING.md) for the checks to run before a pull request.
