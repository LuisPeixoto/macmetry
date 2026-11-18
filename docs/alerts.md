# Alerts

An alert tells you when something needs attention: "the startup disk has under 10% free", "a sensor is above 95 °C".

## A rule

A rule watches one measurement:

- **When**: above or below a threshold, or, for memory and thermal pressure, when a level is reached.
- **For at least**: how long the condition must hold before the alert fires (0 seconds to 30 minutes), so a spike does
  not.
- **Stay quiet afterwards for**: after it fires, the same rule is silent for this long (1 minute to a day).

An alert fires once when the condition has held long enough, and is marked resolved when it clears. A measurement your
Mac does not have never fires an alert: a desktop does not get a "battery below 15%" alert.

## The suggested alerts

Only the two that protect your Mac start on:

| Alert | Default |
|---|---|
| Startup disk almost full | below 10% free, for 1 minute, **on** |
| Very hot sensor | above 95 °C, for 30 seconds, **on** |
| CPU busy for a while | above 90%, for 2 minutes, off |
| Memory pressure is critical | critical level, for 30 seconds, off |
| The Mac is slowing down to cool off | heavy thermal pressure, for 30 seconds, off |
| Battery is low | below 15%, off |

**Alerts > Restore the suggested alerts** brings these back.

## Notifications

When an alert fires macmetry shows a system notification. Turn this off in **Alerts** to keep alerts on screen only.
macOS may ask for permission the first time. Notifications are local: nothing is sent anywhere.

## How it works

The engine is a small state machine driven by each reading's own timestamp. Editing a rule keeps an active alert
active, so changing a threshold does not send a second notification for the same problem. Rules live in
`settings.json` in the data folder.
