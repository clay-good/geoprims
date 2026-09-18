## Why

Coding agents doing spatial or aerospace work today make three kinds of mistakes. They hallucinate formulas: haversine where an ellipsoidal geodesic is needed, ISA temperature where OAT is needed, 35 mm-equivalent focal length in GSD. They install heavy native GIS stacks. Or they call token-gated web APIs. No MCP server in the official registry offers offline geodesy, E6B, photogrammetry, or COGO math (see `docs/research/04`).

geoprims can give agents the exact calculators the website uses: deterministic, cited, offline, and safe to run with no network.

The product has exactly two surfaces: the static website for humans and this local MCP server for agents. There is no separate CLI or public library. The server's `run` and `pipeline` tools cover scripted use, and the website covers CSV batch work.

Exposing 800 tools directly would cost an estimated 120K–320K tokens of schemas and degrade tool selection. Some clients also cap tool counts (VS Code at 128 per request; Cursor reportedly lower). The server is therefore designed for discovery, not enumeration.

Depends on: `establish-platform-foundation`. It serves whatever domain tools are built.

## What Changes

- **A local MCP server** (`@geoprims/mcp`, stdio, Node.js). It loads the same Wasm modules as the website through the internal runtime package and targets MCP spec `2026-07-28`, while staying compatible with `2025-11-25` and `2025-06-18` clients.
- **Five meta-tools by default:** `geoprims_search`, `geoprims_describe`, `geoprims_run`, `geoprims_pipeline`, and `geoprims_convert_units`.
- **Opt-in direct toolsets** (≤ 40 tools each) for clients with native tool search or focused workflows.
- **Catalog resources and workflow prompts.**
- **Offline by default.** A single opt-in flag allows verified downloads of large data tiles.
- **Distribution:** npm (`npx -y @geoprims/mcp`, trusted publishing with provenance), an MCPB bundle for one-click desktop install, and the MCP Registry (`com.geoprims/mcp`).
- **An agent-evaluation benchmark** that gates releases.

## Capabilities

### New Capabilities

- `agent/mcp-server`: Local MCP server behavior, meta-tools, toolsets, resources, prompts, limits, security, and distribution.

### Modified Capabilities

None.

## Non-goals

- No CLI, no public JavaScript library, no Python/Rust/Go bindings.
- No hosted or remote MCP endpoint run by geoprims. It would be server-side compute and would see user inputs.
- No WebMCP (in-page browser agent tools) for now. It is a W3C Community Group draft in a Chrome origin trial; revisit when it ships broadly.
- No filesystem writes except the opt-in asset cache. No live data fetching.

## Impact

- New package: `packages/mcp` (published). It depends on the internal, unpublished `packages/runtime` from the foundation change.
- New release artifacts: the npm package, a `.mcpb` bundle, and a `server.json` registry entry.
- Adds MCP conformance and agent-evaluation jobs to CI.
