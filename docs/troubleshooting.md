# Troubleshooting

## macOS says the app cannot be opened

The builds are not signed with an Apple Developer ID yet. Right-click the app and choose **Open**, or run
`xattr -dr com.apple.quarantine /Applications/macmetry.app` once. See [Distribution](distribution.md).

## There is no battery, temperature, power or GPU card

Cards appear only for what your Mac reports; see [Sensors, power and what each Mac can report](sensors.md). Run
`macmetry snapshot --json` and look at `capabilities` to see what macmetry found.

## Temperatures or power are missing on an Apple silicon Mac

Those use private macOS frameworks that are only confirmed on the Macs they were tested on. Please open an issue with the
output of `macmetry snapshot --json`, and, if you can, run `tools/capture-fixtures.sh` and attach the result after
reading through it.

## The CPU frequency does not change on my Intel Mac

macOS gives the real frequency only to administrators, so macmetry shows the nominal one and labels it so.

## Some processes are missing

Without administrator rights macOS lets a program see only the current user's processes. Activity Monitor uses a
privileged helper for the rest.

## The history is empty

History is recorded while macmetry runs. Check **Settings > Keep a history**, and give it a minute. From a terminal,
`macmetry history` says so when there is no database yet, and `macmetry record` creates one.

## The menu bar item is missing

Check **Settings > Show in the menu bar**. On a Mac with a notch the item can hide behind it when the menu bar is full;
quit a few other menu bar items or use a tool to rearrange them.

## No notifications

macOS needs to allow notifications for macmetry (System Settings > Notifications). Turn **Alerts > Show
notifications** on too.

## macmetry uses more CPU than I expected

Faster refresh costs more. **Settings > Refresh every** to 2 or 5 seconds lowers it. With the window closed macmetry
reads only what the menu bar, history and alerts need.

## An MCP client cannot connect

Run `macmetry mcp` yourself. It should print nothing and wait. Use the full path to the binary in the client's
configuration, and check the client's log for the launch error.

## Reporting a problem

Open an issue with your Mac and macOS version, the macmetry version and the output of `macmetry snapshot --json`
(remove anything you would rather not share; process names can be personal). For a security problem, use
[private reporting](../SECURITY.md).
