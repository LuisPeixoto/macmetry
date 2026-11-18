# MCP server

macmetry includes a [Model Context Protocol](https://modelcontextprotocol.io) server, so an AI assistant can answer
questions such as "why is my Mac slow?" from real readings.

```bash
macmetry mcp
```

It speaks MCP over standard input and output and stops when the assistant disconnects.

## Add it to an assistant

For Claude Code:

```bash
claude mcp add macmetry -- /usr/local/bin/macmetry mcp
```

For other clients, add a server whose command is the path to `macmetry` and whose argument is `mcp`. In a JSON
configuration that looks like:

```json
{ "mcpServers": { "macmetry": { "command": "/usr/local/bin/macmetry", "args": ["mcp"] } } }
```

## Every tool only reads

Nothing the server offers changes your Mac. Every tool is marked read-only and non-destructive, and a test fails if one
is not. The tools are listed, with their arguments, in [MCP tools](mcp-tools.md), which is generated from the server
itself and checked in CI so it cannot go stale.

| Tool | Answers |
|---|---|
| `system_info` | What Mac is this, and what can it report? |
| `snapshot` | How is it doing right now? (use `sections` to ask only for what you need) |
| `processes` | What is using the CPU, memory or disk? |
| `sensors` | Is it hot, are the fans running, is it throttling? |
| `battery` | How is the battery? |
| `history_query` | What happened over the last hour or day? |
| `list_metrics` | Which metrics exist? |
| `watch_start`, `watch_read`, `watch_stop` | What changes while I do something? |

There are also **prompts** for common questions (`why_is_my_mac_slow`, `whats_heating_up`, `battery_health`,
`summarize_recent_activity`) and **resources** (`macmetry://guide`, `system`, `snapshot` and `metrics`).

## What the assistant is not sent

- **Network addresses** of your interfaces are removed from every snapshot. Speeds and totals are kept.
- The server **never writes** to the history; it opens it read-only.
- Process names and users are included, because they are what explains a slow Mac. If that matters to you, ask for
  `sections` without `processes`.

## Limits

| Limit | |
|---|---|
| Watches at once | 4 |
| Readings a watch keeps | 10 to 600 |
| A watch nobody reads | stops after 5 minutes |
| Longest a watch runs | 1 hour |
| History points per query | 2 to 500 |
| Text a tool returns | about 60,000 characters |

Arguments are checked first and refused with a reason the assistant can use.
