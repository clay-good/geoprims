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

Replace `/abs/path/geoprims` with your clone's path.

| Client | Setup |
|---|---|
| Claude Code | `claude mcp add geoprims -- node /abs/path/geoprims/mcp/server.mjs` |
| Claude Desktop | In `claude_desktop_config.json`: `{"mcpServers": {"geoprims": {"command": "node", "args": ["/abs/path/geoprims/mcp/server.mjs"]}}}` |
| VS Code | In `.vscode/mcp.json`: `{"servers": {"geoprims": {"type": "stdio", "command": "node", "args": ["/abs/path/geoprims/mcp/server.mjs"]}}}` |
| Cursor | In `.cursor/mcp.json`: `{"mcpServers": {"geoprims": {"command": "node", "args": ["/abs/path/geoprims/mcp/server.mjs"]}}}` |
| Windsurf | In `~/.codeium/windsurf/mcp_config.json`: `{"mcpServers": {"geoprims": {"command": "node", "args": ["/abs/path/geoprims/mcp/server.mjs"]}}}` |

The `npx -y @geoprims/mcp` path works once the package is published (not yet).

## Tools

| Tool | What it does |
|---|---|
| `geoprims_search` | Ranks tools for a query with the same core ranker as the website's command palette. Experimental tools are hidden unless `includeExperimental` is true. |
| `geoprims_describe` | Returns manifests for up to 20 ids at `summary`, `schema`, or `examples` detail |
| `geoprims_run` | Runs a tool. With no `args` it runs the worked example. `units` picks an output unit profile. |
| `geoprims_pipeline` | Runs up to 20 steps. A bound `{value, unit}` carries its unit, so conversions happen automatically. |
| `geoprims_convert_units` | Exact unit conversion, finding the quantity from the units |

Resources: `geoprims://catalog` and `geoprims://tool/{id}` (the manifest plus its golden vectors).

`structuredContent` is the core's result envelope byte for byte (`{"ok":true,"result":…,"meta":…}`), and errors come back with `isError: true` and a geoprims error code.

## Options

| Option | Effect |
|---|---|
| `--timeout=<ms>` | Per-call time limit (default 10,000). A call over it returns `LIMIT_EXCEEDED`, and the server keeps serving. |
| `--debug` | Logs argument values to stderr (off by default) |
| `--allow-asset-download` | Accepted. No downloadable assets exist yet. |

## Not built yet

`geoprims_report_problem` (it needs the core's permalink encoder), workflow prompts, direct toolsets (`--toolsets`, `--no-meta`), paginated collections, the `summary` sentence and `explain` trace in run results, npm and MCPB packaging, and the MCP Inspector CI job. Protocol negotiation is tested with synthetic handshakes for `2025-06-18`, `2025-11-25`, and `2026-07-28` but not yet against recorded real clients.

## Tests

```bash
node --test mcp/server.test.mjs
```

`mcp/surface.json` is the golden surface: tools, resources, templates, and prompts. The test fails on any change. Regenerate it with `UPDATE_SURFACE=1` after reviewing the diff.
