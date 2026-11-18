# Command line

`macmetry` is one binary with the same readings as the app. Arguments are checked before anything is measured, so a
mistake fails at once with a sentence that says what to change.

```
macmetry <command> [options]
```

| Command | What it does |
|---|---|
| `info` | The model, chip, cores, memory and macOS |
| `snapshot` | Measure everything once and print it |
| `watch` | A live view that refreshes in place; `Ctrl-C` leaves |
| `top` | The busiest processes |
| `sensors` | Temperatures and fans |
| `battery` | Charge, health, cycles and power |
| `history` | A chart of one metric from the recorded history |
| `export` | Write recorded history to a CSV or JSON file |
| `record` | Record the history from the terminal, without the app |
| `metrics` | List the metric keys |
| `mcp` | Start the [MCP server](mcp.md) on standard input and output |

Most commands take `--json`, which prints the same data machine-readably.

## snapshot

```bash
macmetry snapshot                       # human readable
macmetry snapshot --json --processes 10 # everything, as JSON
```

| Option | Default | |
|---|---|---|
| `--wait-ms` | 1000 | How long to measure first (200 to 10000). Rates need a little time to mean anything. |
| `--processes` | 5 | How many of the busiest processes to include (0 to 200) |

`--json` prints `{ "system": ..., "capabilities": ..., "snapshot": ... }`. `capabilities` says what this Mac can report.

## watch

`macmetry watch --interval 2` redraws every 2 seconds (0.5 to 60). `--count 10` stops after ten refreshes, which is
handy in scripts. Piped to a file, it appends instead of redrawing.

## top

```bash
macmetry top --sort memory --limit 20
macmetry top --filter chrome
```

`--sort` is `cpu`, `memory`, `disk-read`, `disk-write` or `name`. CPU is a percentage of one core. Only your own
processes are visible without administrator rights.

## history and record

`history` charts one metric in the terminal:

```bash
macmetry history --metric cpu.total --since 6h --points 80
```

`--since` takes `30s`, `15m`, `6h` or `7d` (up to 31 days). The history exists only if the app, or `macmetry record`,
was running:

```bash
macmetry record --interval 5     # Ctrl-C to stop
```

## export

```bash
macmetry export --metric cpu.total --metric temp.max --from -6h -o cpu-and-heat.csv
macmetry export --metric net.rx_bps --format json --from 2024-02-29T10:00:00Z --to 2024-02-29T12:00:00Z
```

| Option | |
|---|---|
| `--metric` | A key from `macmetry metrics`. Repeat it for several. |
| `--from`, `--to` | `-6h` (six hours ago), `now`, or an RFC 3339 time such as `2024-02-29T10:00:00Z` or `...+02:00` |
| `--format` | `csv` (default) or `json` |
| `--range` | Add the lowest and highest of each point to the CSV |
| `--points` | At most this many points (2 to 10000, default 1000) |
| `-o, --output` | The file to write; the terminal when omitted |
| `--force` | Replace the file if it exists. Without it an existing file is never overwritten. |

The CSV has `timestamp` (UTC), `timestamp_ms` and one column per metric. A value that was not available is an empty
cell.

## Data folder

Everything macmetry writes goes to `~/Library/Application Support/macmetry`. Set `MACMETRY_HOME` to use another folder.
The commands that read the history open it read-only.
