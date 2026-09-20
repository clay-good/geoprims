# geoprims MCP server

A local, offline MCP server that runs the same WebAssembly calculators as geoprims.com. It has zero npm dependencies, uses stdio only, and makes no network requests.

## Run it

Release tags will ship prebuilt files in `mcp/dist/`, so a tagged clone runs with Node alone. No release is tagged yet, so for now build from source (Rust via rustup, binaryen `wasm-opt`, and Node 22 or newer):

```bash
npm run build
```

```bash
node mcp/server.mjs
```

Without built files, the server prints how to get them and exits with code 1.

## Add it to a client

Replace `/abs/path/geoprims` with your clone's path. The same snippets are on [geoprims.com/agents](https://geoprims.com/agents/), and a test launches the server exactly as each one says.

### Claude Code

Run in a terminal.

```bash
claude mcp add geoprims -- node /abs/path/geoprims/mcp/server.mjs
```

### Claude Desktop

Settings, Developer, Edit Config: claude_desktop_config.json.

```json
{
  "mcpServers": {
    "geoprims": {
      "command": "node",
      "args": [
        "/abs/path/geoprims/mcp/server.mjs"
      ]
    }
  }
}
```

### VS Code

.vscode/mcp.json in your workspace.

```json
{
  "servers": {
    "geoprims": {
      "type": "stdio",
      "command": "node",
      "args": [
        "/abs/path/geoprims/mcp/server.mjs"
      ]
    }
  }
}
```

### Cursor

.cursor/mcp.json in your project, or ~/.cursor/mcp.json.

```json
{
  "mcpServers": {
    "geoprims": {
      "command": "node",
      "args": [
        "/abs/path/geoprims/mcp/server.mjs"
      ]
    }
  }
}
```

### Windsurf

~/.codeium/windsurf/mcp_config.json.

```json
{
  "mcpServers": {
    "geoprims": {
      "command": "node",
      "args": [
        "/abs/path/geoprims/mcp/server.mjs"
      ]
    }
  }
}
```

For the `npx` path (once `@geoprims/mcp` is published), use `"command": "npx", "args": ["-y", "@geoprims/mcp"]` instead, or `claude mcp add geoprims -- npx -y @geoprims/mcp`.

## Tools

| Tool | What it does |
|---|---|
| `geoprims_search` | Ranks tools for a query with the same core ranker as the website's command palette. Experimental tools are hidden unless `includeExperimental` is true. |
| `geoprims_describe` | Returns manifests for up to 20 ids at `summary`, `schema`, or `examples` detail, including each tool's citations, the constants it assumes with their sources, and the limitation a simplified tool declares |
| `geoprims_run` | Runs a tool. With no `args` it runs the worked example. `units` picks an output unit profile. `explain: true` adds the tool's work: each step's formula, the formula with your values in it, and what it came to. Every result carries `meta.references`: the publisher, title, edition, and locator of each source behind the answer |
| `geoprims_pipeline` | Runs up to 20 steps. A bound `{value, unit}` carries its unit, so conversions happen automatically. |
| `geoprims_convert_units` | Exact unit conversion, finding the quantity from the units |
| `geoprims_report_problem` | Prepares (never sends) a problem report: the payload, a geoprims.com link that reopens the tool with the inputs and the report form filled in, and a GitHub issue link |

Resources: `geoprims://catalog` and `geoprims://tool/{id}` (the manifest plus its golden vectors).

`structuredContent` is the core's result envelope byte for byte (`{"ok":true,"result":…,"summary":…,"meta":…}`, where `summary` is the plain-language sentence the website shows), and errors come back with `isError: true` and a geoprims error code. Lists longer than `output.maxItems` (default 1,000, up to 10,000) come back one page at a time from `output.offset`, with `page.<list>` giving `total`, `offset`, `returned`, and `truncated` (and a bounding box when the items carry latitude and longitude). The H3 polygon fill pages in the core and reports the covered area and bounds of every cell, so an agent can reason about a 250,000-cell fill from one page. Results from aviation, drone, navigation, and magnetic tools carry `meta.notice` ("Planning and education aid. Not for primary navigation."), and tools with operating context an agent should relay put it in `meta.context`. For example, magnetic declination reports the model, epoch used, validity window, declination uncertainty, and whether the point is in a compass blackout or caution zone.

## Prompts

Five workflow prompts turn a few arguments into one `geoprims_pipeline` call, with what to check and relay: `preflight-performance` (pressure and density altitude, runway wind components against a crosswind limit), `photogrammetry-mission` (flight height for a target GSD, trigger interval and line spacing, blur-free exposure), `traverse-closure` (misclosure and precision ratio), `coordinate-conversion-audit` (parse any notation, convert to UTM and back), and `h3-resolution-choice` (resolution for a target area and the real cell area at a point). A test runs each prompt's chain end to end.

## Options

| Option | Effect |
|---|---|
| `--toolsets=<name,...>` | Also lists each stable tool in the named toolsets as its own MCP tool, named `gp_` plus the id with dots as underscores (`gp_geodesy_utm_forward`). Toolsets: `geodesy-core`, `navigation`, `e6b`, `atmosphere`, `drone-mapping`, `survey-cogo`, `indexing`. Only stable tools join, so `e6b`, `drone-mapping`, and `survey-cogo` are empty until tools there are promoted. |
| `--no-meta` | With `--toolsets`, lists only the direct tools |
| `--timeout=<ms>` | Per-call time limit (default 10,000). A call over it returns `LIMIT_EXCEEDED`, and the server keeps serving. |
| `--debug` | Logs argument values to stderr (off by default) |
| `--allow-asset-download` | Accepted. No downloadable assets exist yet. |

## Not built yet

The `explain` trace in run results, npm and MCPB packaging, and the MCP Inspector CI job. Protocol negotiation is tested with synthetic handshakes for `2025-06-18`, `2025-11-25`, and `2026-07-28` but not yet against recorded real clients.

## Tests

```bash
node --test mcp/server.test.mjs mcp/network.test.mjs
```

The network audit runs every tool's worked example with networking denied by the OS (skipped where no sandbox is available) and fails on any connection or DNS attempt.

`mcp/surface.json` is the golden surface: tools, resources, templates, and prompts. The test fails on any change. Regenerate it with `UPDATE_SURFACE=1` after reviewing the diff.
