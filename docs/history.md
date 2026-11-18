# History

macmetry can keep a history, so you can see what happened before you opened it. It is on by default and can be turned
off in **Settings**.

## What is kept

| Detail | For how long |
|---|---|
| One reading a second ("raw") | 24 hours |
| One-minute average, lowest and highest | 30 days |

Each one-minute record holds the average, the lowest and the highest value, and how many readings it came from, so a
short spike is not lost when a long range is shown, and averages stay correct when minutes are combined.

History is written while macmetry runs, including from the menu bar with the window closed. `macmetry record` does the
same from a terminal, without the app.

A metric your Mac does not have is **empty** in the history, not zero.

## The History view

Pick a range (1 hour to 30 days) and up to 8 measurements. Measurements in **different units get separate charts**, so
a chart never has two vertical scales. With one measurement, the faint band behind the line is its lowest and highest
value.

## Export

**CSV** and **JSON**, from the History view or `macmetry export`. The file is written only after you choose where to
save it. A measurement with no value at a time is an empty cell. See [Command line](cli.md).

## Where it lives

`~/Library/Application Support/macmetry/history.sqlite`. Set `MACMETRY_HOME` to use another folder. The command line
and the MCP server open it **read-only**, so they can never change it. **Settings > Delete the history** removes
everything.

The history holds numbers and timestamps. It does not hold process names or network addresses.
