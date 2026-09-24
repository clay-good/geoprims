# geoprims MCP server

A local, offline MCP server that runs the same WebAssembly calculators as geoprims.com. It has zero npm dependencies, uses stdio only, and makes no network requests.

## Run it

Release tags carry the prebuilt calculators in `mcp/dist/`, so a clone of a tag runs with Node 22 or newer and nothing else. The server is not published to npm; the repository is the way to get it.

```bash
git clone --branch v0.1.0 --depth 1 https://github.com/clay-good/geoprims
```

```bash
node geoprims/mcp/server.mjs
```

Each release on [GitHub](https://github.com/clay-good/geoprims/releases) also has a zip of the same files and a `.mcpb` bundle that Claude Desktop installs in one click. To build from source instead, clone `main` and run `npm run build` (Rust via rustup, binaryen `wasm-opt`, and Node 22 or newer).

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

### Desktop bundle

`npm run build:mcpb` writes `dist/mcpb/geoprims-<version>.mcpb`, a one-click bundle for desktop hosts: the server, its manifest, and the whole `mcp/dist` it reads, so the installed extension is as offline as the checkout. The zip is built byte-for-byte reproducibly — fixed entry order, fixed timestamps — so the same source gives the same digest, which the release notes carry.

The bundle's one setting is the same `--toolsets` the command line takes: leave it empty for the six meta-tools, or name domains to expose their stable tools directly.

## Tools

| Tool | What it does |
|---|---|
| `geoprims_search` | Ranks tools for a query with the same core ranker as the website's command palette. Experimental tools are hidden unless `includeExperimental` is true. |
| `geoprims_describe` | Returns manifests for up to 20 ids at `summary`, `schema`, or `examples` detail, including each tool's citations, the constants it assumes with their sources, and the limitation a simplified tool declares |
| `geoprims_run` | Runs a tool. With no `args` it runs the worked example. `units` picks an output unit profile. `explain: true` adds the tool's work: each step's formula, the formula with your values in it, and what it came to. Every result carries `meta.references`: the publisher, title, edition, and locator of each source behind the answer |
| `geoprims_pipeline` | Runs up to 20 steps. A bound `{value, unit}` carries its unit, so conversions happen automatically. |
| `geoprims_convert_units` | Exact unit conversion, finding the quantity from the units |
| `geoprims_report_problem` | Prepares (never sends) a problem report: the payload, and a geoprims.com link that reopens the tool with the inputs and the report form filled in |

Resources: `geoprims://catalog` and `geoprims://tool/{id}` (the manifest plus its golden vectors).

`structuredContent` is the core's result envelope byte for byte (`{"ok":true,"result":…,"summary":…,"meta":…}`, where `summary` is the plain-language sentence the website shows), and errors come back with `isError: true` and a geoprims error code. Lists longer than `output.maxItems` (default 1,000, up to 10,000) come back one page at a time from `output.offset`, with `page.<list>` giving `total`, `offset`, `returned`, and `truncated` (and a bounding box when the items carry latitude and longitude). The H3 polygon fill pages in the core and reports the covered area and bounds of every cell, so an agent can reason about a 250,000-cell fill from one page. Results from aviation, drone, navigation, and magnetic tools carry `meta.notice` ("Planning and education aid. Not for primary navigation."), and tools with operating context an agent should relay put it in `meta.context`. For example, magnetic declination reports the model, epoch used, validity window, declination uncertainty, and whether the point is in a compass blackout or caution zone.

## Prompts

Five workflow prompts turn a few arguments into one `geoprims_pipeline` call, with what to check and relay: `preflight-performance` (pressure and density altitude, runway wind components against a crosswind limit), `photogrammetry-mission` (flight height for a target GSD, trigger interval and line spacing, blur-free exposure), `traverse-closure` (misclosure and precision ratio), `coordinate-conversion-audit` (parse any notation, convert to UTM and back), and `h3-resolution-choice` (resolution for a target area and the real cell area at a point). A test runs each prompt's chain end to end.

## Options

| Option | Effect |
|---|---|
| `--toolsets=<name,...>` | Also lists each stable tool in the named toolsets as its own MCP tool, named `gp_` plus the id with dots as underscores (`gp_geodesy_utm_forward`). Toolsets: `geodesy-core`, `projections`, `navigation`, `e6b`, `atmosphere`, `drone-mapping`, `survey-cogo`, `indexing`. Only stable tools join, up to 40 per toolset. `projections` holds the projections you set the parameters of (Lambert, Albers, polar stereographic, Web Mercator, and others) beside UTM, UPS, and State Plane. |
| `--no-meta` | With `--toolsets`, lists only the direct tools |
| `--timeout=<ms>` | Per-call time limit (default 10,000). A call over it returns `LIMIT_EXCEEDED`, and the server keeps serving. |
| `--debug` | Logs argument values to stderr (off by default) |
| `--allow-asset-download` | Accepted. No downloadable assets exist yet. |

## Not built yet

npm and MCPB packaging, and the MCP Inspector CI job. Protocol negotiation is tested with synthetic handshakes for `2025-06-18`, `2025-11-25`, and `2026-07-28` but not yet against recorded real clients.

## Tests

```bash
node --test mcp/server.test.mjs mcp/progress.test.mjs mcp/network.test.mjs
```

For a long `tools/call`, include `_meta.progressToken` to receive `notifications/progress` with elapsed milliseconds. On stdio, `notifications/cancelled` with the call's `requestId` stops its worker; the canceled request sends no result or error, and later calls still run. `mcp/progress.test.mjs` checks both a running call and a call canceled before it starts.

The network audit runs every tool's worked example with networking denied by the OS and fails on any connection or DNS attempt. It skips when the host cannot apply the network-denying policy, including inside a restricted container. If the sandboxed server exits before replying, the test reports that exit immediately.

`mcp/surface.json` is the golden surface: tools, resources, templates, and prompts. The test fails on any change. Regenerate it with `UPDATE_SURFACE=1` after reviewing the diff.

`tools/mcp/parity.test.mjs` sweeps every tool on the hero checklist and fails when this server and the website disagree on the sentence, the answer, the citations, or a line of the work.
