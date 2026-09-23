## Purpose

Gives LLM agents local, offline, deterministic access to the full geoprims catalog through the Model Context Protocol, using a small discovery-first tool surface that fits within client tool limits and context budgets.

## ADDED Requirements

### Requirement: Local stdio server using the shared core
The MCP server SHALL run locally over the stdio transport, SHALL execute tools through the same Wasm modules as the website (per compute-core), and SHALL NOT offer a network-listening transport by default.

#### Scenario: Same result as website
- **WHEN** an agent runs `navigation.geodesic.inverse` with the same inputs as a website permalink
- **THEN** the `structuredContent` values are byte-identical to the website's copied JSON result

#### Scenario: No listening socket
- **WHEN** the server starts with default options
- **THEN** it opens no TCP or UDP socket

### Requirement: Protocol version support
The server SHALL implement MCP specification `2026-07-28` (stateless requests with per-request `_meta` protocol version and capabilities, `server/discover`) and SHALL remain interoperable with clients using `2025-11-25` and `2025-06-18` via version negotiation.

#### Scenario: Older client
- **WHEN** a client initializes with protocol version `2025-06-18`
- **THEN** the server completes the legacy handshake and serves the same tools and results

### Requirement: Default meta-tool surface
By default the server SHALL expose exactly these tools, in this deterministic order:
1. `geoprims_search` — input `{query, domain?, limit? (default 10, max 50), includeExperimental? (default false)}`; returns ranked `{id, title, summary, domain, stability, prefill?}` entries, where `prefill` holds arguments extracted from a natural-language query (per `discovery/natural-language-prefill`). `geoprims_run` accepts experimental ids but attaches warning `EXPERIMENTAL_TOOL`.
2. `geoprims_describe` — input `{ids: string[] (max 20), detail: "summary" | "schema" | "examples"}`; returns manifests at the requested detail, including input/output JSON Schemas, units, accuracy, citations, limitation text, and related tools for `schema`, and the worked example for `examples`.
3. `geoprims_run` — input `{id, args?, units?, explain?, output?: {maxItems?, offset?}}`; validates `args` against the tool's input schema, executes, and returns the result with `summary` (the plain-language sentence), `meta.references`, and, when `explain` is true, the step-by-step `trace`. With no `args`, it runs the tool's worked example.
4. `geoprims_pipeline` — input `{steps: [{id, args, bind?: {<inputPointer>: "<stepIndex>:<outputPointer>"}}] (max 20 steps)}`; runs a chain without model round trips.
5. `geoprims_convert_units` — input `{value, from, to}`; converts using the unit registry.
6. `geoprims_report_problem` — prepares, but never sends, a problem report (per `feedback/triage-and-corrections`).

The total serialized size of the default `tools/list` result SHALL be at most 6,000 tokens (measured with a documented tokenizer approximation of 4 characters per token).

#### Scenario: Default tool list
- **WHEN** a client calls `tools/list` with default server options
- **THEN** exactly the six meta-tools are returned in the specified order

#### Scenario: Search then run
- **WHEN** an agent searches "density altitude", describes the top id at `schema` detail, and runs it with valid args
- **THEN** each call succeeds and the run result contains `meta.model` and `meta.accuracy`

### Requirement: Tool annotations
Every tool the server exposes SHALL declare `title` and annotations `readOnlyHint: true`, `destructiveHint: false`, `idempotentHint: true`, and `openWorldHint: false`, and SHALL declare an `outputSchema` with results returned as `structuredContent` plus an equivalent JSON text content block.

#### Scenario: Annotations present
- **WHEN** a client lists tools
- **THEN** every tool includes the four annotations with the specified values and an `outputSchema`

### Requirement: Optional direct toolsets
The server SHALL support opt-in direct toolsets selected by a server launch flag or bundle configuration (`--toolsets=<name,...>`), where each toolset exposes up to 40 stable catalog tools as first-class MCP tools, named by replacing dots with underscores and prefixing `gp_` (e.g. `gp_aviation_altimetry_density-altitude`, subject to the 64-character name rule below). Predefined toolsets SHALL include at least: `geodesy-core`, `navigation`, `e6b`, `atmosphere`, `drone-mapping`, `survey-cogo`, `indexing`. The meta-tools SHALL remain available alongside toolsets unless `--no-meta` is given.

#### Scenario: E6B toolset
- **WHEN** the server starts with `--toolsets=e6b`
- **THEN** `tools/list` returns the six meta-tools plus at most 40 E6B tools

#### Scenario: Unknown toolset
- **WHEN** the server starts with `--toolsets=bogus`
- **THEN** it exits with a non-zero code and a message listing valid toolset names

### Requirement: Errors are recoverable by the model
Input validation failures and tool errors SHALL be returned as tool results with `isError: true` and a structured body containing the geoprims error `code`, `message`, `field`, and `hint` (per tool-contract), so the model can correct and retry. Unknown meta-tool names SHALL be JSON-RPC errors. For an unknown catalog id in `geoprims_run`, the error SHALL include up to 3 closest ids from search.

#### Scenario: Wrong id suggestion
- **WHEN** an agent runs id `aviation.density-altitude`
- **THEN** the result has `isError: true`, code `UNSUPPORTED`, and suggests `aviation.altimetry.density-altitude`

### Requirement: Output size control
Results that contain large collections (cells, vertices, rows) SHALL be paginated: `geoprims_run` SHALL return at most `output.maxItems` (default 1,000, maximum 10,000) items per collection with `total`, `offset`, and `truncated` fields, plus summary statistics (count, bounding box, area where meaningful) so the agent can reason without the full list.

#### Scenario: Large polyfill
- **WHEN** an agent polyfills a county at H3 resolution 10 producing 250,000 cells
- **THEN** the result returns 1,000 cells, `total: 250000`, `truncated: true`, and the covered area and bounding box

### Requirement: Resources and prompts
The server SHALL expose resources `geoprims://catalog` (compact index of all ids, titles, summaries) and `geoprims://tool/{id}` (full manifest with docs text, worked example, citations, and golden test vectors), with `ttlMs` and `cacheScope: "public"` on list results. It SHALL expose workflow prompts at minimum: `preflight-performance`, `photogrammetry-mission`, `traverse-closure`, `coordinate-conversion-audit`, and `h3-resolution-choice`.

#### Scenario: Read tool resource
- **WHEN** a client reads `geoprims://tool/geodesy.utm.forward`
- **THEN** it receives the manifest, accuracy statement, references, and worked example

### Requirement: Descriptions carry caveats
Tool descriptions and results for aviation, drone, navigation, magnetic, and datum tools SHALL carry the model/epoch and the "not for navigation" caveat in the result `meta` so agents can relay it, and descriptions SHALL be static text that never interpolates user input.

#### Scenario: Magnetic caveat
- **WHEN** an agent runs magnetic declination
- **THEN** the result `meta` includes the model (`WMM2025`), epoch used, validity window, uncertainty, and blackout/caution-zone status

### Requirement: Offline assets and opt-in fetching
The server SHALL work fully offline with bundled small assets (`wmm2025`, `wmmhr2025`, `igrf14`, `egm96-15`, `crs-registry`, `ne-110m`, `deformation-zones`, `leap-seconds`, `tzdb`), at most 6 MB in total. Tools needing un-cached assets SHALL return `ASSET_UNAVAILABLE` naming the dataset, the tile, its download size, and how to enable downloads. On-demand fetching from the geoprims asset origin SHALL be enabled only with `--allow-asset-download`, SHALL use the same coarse tiles as the website, and SHALL verify integrity.

#### Scenario: Geoid tile missing
- **WHEN** an agent requests an EGM2008-1 geoid height and the tile is not cached and downloads are not allowed
- **THEN** the result has `isError: true`, code `ASSET_UNAVAILABLE`, the tile id and size, and a hint to restart the server with `--allow-asset-download`, and suggests the EGM96-15 result with its lower accuracy stated

#### Scenario: Download allowed
- **WHEN** the server was started with `--allow-asset-download` and the same request is made
- **THEN** the server fetches the single verified tile from the geoprims asset origin, caches it under the OS cache directory, and returns the EGM2008-1 result

### Requirement: Resource limits and robustness
The server SHALL enforce per-call wall-clock timeouts (default 10 s, configurable), the tool-declared input limits, a maximum request size of 10 MB, and SHALL survive any single tool failure (Wasm trap) without exiting. It SHALL write logs only to stderr and SHALL NOT log argument values unless `--debug` is set.

#### Scenario: Timeout
- **WHEN** a call exceeds the timeout
- **THEN** it is canceled and returns `isError: true` with code `LIMIT_EXCEEDED` naming the timeout, and the server keeps serving

### Requirement: No telemetry
The server SHALL NOT send telemetry, update checks, or any network traffic except opt-in asset downloads.

#### Scenario: Network audit
- **WHEN** the server runs the full agent-evaluation suite under a network sandbox with no allowed hosts
- **THEN** every tool not requiring an un-cached asset succeeds

### Requirement: Runs from a GitHub clone with nothing to install
A developer SHALL be able to run the server from a clone of the public repository at any release tag with only Node.js (active LTS) installed: `git clone --depth 1 --branch vX.Y.Z https://github.com/clay-good/geoprims && node geoprims/mcp/server.mjs`. Release tags SHALL contain the prebuilt Wasm modules, the compiled internal runtime, the catalog, and bundled assets under `mcp/dist/`, so no Rust toolchain, `npm install`, or network access is needed. The server SHALL have zero runtime npm dependencies. A clone of an untagged commit without built artifacts SHALL print a one-line message explaining how to build or check out a tag, and exit non-zero.

#### Scenario: Clone and run
- **WHEN** a developer with only Node.js clones a release tag and adds `{"command": "node", "args": ["/abs/path/geoprims/mcp/server.mjs"]}` to their client
- **THEN** the client lists the six meta-tools, with no install step and no network access

#### Scenario: Untagged checkout
- **WHEN** the server is started from a main-branch clone without built artifacts
- **THEN** it prints "Built files missing: check out a release tag (git checkout vX.Y.Z) or run npm run build (requires Rust)" and exits with code 1

#### Scenario: Verify before running
- **WHEN** a developer runs the documented verification command on a release tag
- **THEN** the SHA-256 digests of the Wasm modules match the release notes and the site's published digests

### Requirement: Setup instructions for major clients
The repository README and the site's "Use with agents" page SHALL give copy-paste setup for Claude Code (`claude mcp add`), Claude Desktop (JSON and MCPB), VS Code (`.vscode/mcp.json` with the `servers` key), Cursor, and Windsurf, for both the clone path and the npx path. Each SHALL be tested in CI where a CLI exists and verified manually per release otherwise.

#### Scenario: VS Code key
- **WHEN** a developer copies the VS Code snippet
- **THEN** it uses the top-level `servers` key and a working command

### Requirement: Golden surface file
The server's full surface (tools, schemas, annotations, resources, prompts) SHALL be snapshotted in a committed golden file. CI SHALL fail on any unreviewed difference, and `/.well-known/mcp.json` SHALL be generated from it. CI SHALL also exercise the server through the MCP Inspector CLI.

#### Scenario: Surface drift
- **WHEN** a change alters a tool's input schema without updating the golden file
- **THEN** CI fails showing the diff

### Requirement: Distribution
The server SHALL be distributed through the repository only, not npm or the MCP Registry (owner decision, 2026-09-23). Each release tag SHALL commit the prebuilt `mcp/dist/`, so a clone of the tag, or GitHub's zip of it, runs with Node alone. Each GitHub release SHALL attach an MCPB bundle (`server.type: node`) for one-click desktop install, with its SHA-256 in the release notes. The package SHALL be marked `private` so it cannot be published by accident.

#### Scenario: Clone a release and run
- **WHEN** a user runs `git clone --branch v0.1.0 --depth 1 https://github.com/clay-good/geoprims` and adds `node <path>/mcp/server.mjs` to a client, with only Node 22 installed
- **THEN** the client lists the six meta-tools

### Requirement: Agent-evaluation benchmark
The project SHALL maintain an evaluation set of at least 100 natural-language tasks spanning all domains, each with an expected tool id and expected numeric answer within tolerance, and SHALL report tool-selection accuracy and answer accuracy per release for at least one current frontier model. A release SHALL NOT regress answer accuracy by more than 3 percentage points without a recorded justification.

#### Scenario: Eval report
- **WHEN** the release pipeline runs the agent evaluation
- **THEN** it publishes selection accuracy, answer accuracy, and mean tokens per task

### Requirement: Deterministic, version-bound pagination
Paginated collections SHALL be ordered deterministically (for H3 and S2, ascending cell id; for vertices, input order), and page requests SHALL carry an opaque cursor that binds the tool id, toolVersion, coreVersion, asset versions, and a hash of the arguments. A cursor used with different arguments or versions SHALL return `INVALID_INPUT`. The total size of any paginated result SHALL respect the tool's declared limit (e.g. 5,000,000 H3 cells).

#### Scenario: Stale cursor
- **WHEN** a cursor from one server version is used after an upgrade
- **THEN** the call returns `INVALID_INPUT` stating that the cursor is stale and must be regenerated

### Requirement: Tool names portable across clients
Direct toolset tool names SHALL be at most 64 characters matching `^[a-zA-Z0-9_-]{1,64}$` (the strictest common LLM API rule). Longer names SHALL be shortened deterministically with a hash suffix, and the build SHALL fail if two names collide.

#### Scenario: Long id shortened
- **WHEN** a toolset tool's derived name exceeds 64 characters
- **THEN** it is shortened with a stable hash suffix and remains unique

### Requirement: Server launch options are a stable, documented interface
The launch options (`--toolsets`, `--no-meta`, `--allow-asset-download`, `--timeout`, `--debug`) are server configuration, not a CLI product. They SHALL be documented on the "Use with agents" page, mirrored as MCPB `user_config` fields, and changed only with a deprecation period of at least one minor release.

#### Scenario: Deprecated option
- **WHEN** a renamed option is passed during its deprecation period
- **THEN** the server starts, honors it, and logs a deprecation notice to stderr
