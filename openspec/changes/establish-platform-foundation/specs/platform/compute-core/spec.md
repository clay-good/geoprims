## Purpose

Defines the WebAssembly compute core that executes every geoprims tool identically in the browser for the website and in Node.js for the local MCP server.

## ADDED Requirements

### Requirement: One compute core for every surface
The website and the local MCP server SHALL execute tools through the same compiled WebAssembly modules built from the same source revision. No surface SHALL contain a second implementation of a tool's mathematics.

#### Scenario: Same artifact hashes
- **WHEN** a release is built
- **THEN** the SHA-256 digest of each domain Wasm module shipped to the website equals the digest of the corresponding module in the published MCP server package

#### Scenario: Cross-surface equality
- **WHEN** the golden-vector suite runs in the web app (headless Chromium, WebKit, Gecko) and in Node.js
- **THEN** every vector produces the same serialized result on all four hosts

### Requirement: Domain-split, lazily loaded modules
The core SHALL be split into independently loadable modules by domain (at minimum: `base` (which also serves the `units` domain), `geodesy`, `navigation`, `geometry`, `aviation`, `drone`, `survey`, `indexing`, `raster`). A tool page SHALL load only `base` plus the modules its tool declares. Each domain module SHALL be at most 400 KB compressed (Brotli), and `base` SHALL be at most 120 KB compressed.

#### Scenario: Aviation page does not load indexing
- **WHEN** a user opens `aviation.airspeed.cas-to-tas` directly
- **THEN** the network log shows `base` and `aviation` modules only, and no `indexing` or `raster` module

#### Scenario: Size budget enforced
- **WHEN** a build produces a domain module exceeding its compressed budget
- **THEN** CI fails naming the module, its size, and the budget

### Requirement: Stable host call interface
Each module SHALL expose a uniform call interface: `invoke(toolId, inputJson) -> resultJson` and `invokeBatch(toolId, inputsJson) -> resultsJson`, plus `manifest() -> manifestJson` and `version() -> string`. Data assets SHALL be supplied by the host through a documented asset-provider interface; the core SHALL NOT perform I/O itself.

#### Scenario: Unknown tool id
- **WHEN** `invoke` is called with an id not present in the loaded modules
- **THEN** the result is error `UNSUPPORTED` naming the id and, if the id exists in another module, naming that module

#### Scenario: Asset provided by host
- **WHEN** a tool requires dataset `egm2008-2.5` and the host has not supplied it
- **THEN** the core returns `ASSET_UNAVAILABLE` naming the dataset id and version, and the host is responsible for fetching and retrying

### Requirement: Execution off the main thread in browsers
In the browser, tool execution SHALL run in a dedicated Web Worker. The UI thread SHALL remain responsive (no task longer than 50 ms attributable to tool execution) during any computation.

#### Scenario: Heavy tool keeps UI responsive
- **WHEN** a viewshed over a 1,000 × 1,000 elevation window is running
- **THEN** the command palette still opens within 100 ms of pressing `/`

### Requirement: Cancellation and progress for long operations
Tools whose declared worst case exceeds 100 ms SHALL report progress at least every 250 ms and SHALL be cancelable. Cancellation SHALL stop computation within 100 ms and return no partial result.

#### Scenario: User cancels
- **WHEN** a user changes an input while a long polyfill is running
- **THEN** the running invocation stops within 100 ms (measured from the input event), no partial result from it is displayed or returned, and a new invocation starts with the new input

### Requirement: Performance budgets
For single (non-batch) invocations on a reference mid-tier device (defined in the verification capability), the p95 execution time SHALL be at most 2 ms for closed-form tools, 10 ms for iterative tools (geodesic inverse, projection inverse, airspeed inversions), and SHALL be declared per tool for data-proportional tools. Cold module instantiation SHALL be at most 150 ms for any domain module.

#### Scenario: Budget regression blocks release
- **WHEN** the benchmark suite measures `navigation.geodesic.inverse` at a p95 of 14 ms on the reference profile
- **THEN** CI marks the build failed and reports the regression against the previous release

### Requirement: Memory safety and bounds
The core SHALL cap linear memory growth per invocation at a declared per-tool maximum (default 64 MiB) and SHALL return `LIMIT_EXCEEDED` rather than trapping when a request would exceed it. A Wasm trap SHALL be caught by the host and reported as `INTERNAL` with the tool id, never crashing the page or the MCP server process.

#### Scenario: Trap is contained
- **WHEN** a tool traps due to a defect
- **THEN** the web UI shows an error for that tool only, the worker is restarted, and other tools keep working; the MCP server returns an error result and keeps serving

### Requirement: No threads required
The core SHALL function correctly in single-threaded mode without SharedArrayBuffer. Multi-threaded execution MAY be used only when the page is cross-origin isolated, and results SHALL be identical to single-threaded results.

#### Scenario: Non-isolated context
- **WHEN** the site is embedded or served without cross-origin isolation headers
- **THEN** every tool still runs and returns identical results to the isolated context

### Requirement: Input hardening before execution
Hosts SHALL reject, before calling the core: JSON numbers that overflow binary64 to ±Infinity, duplicate object keys, nesting deeper than 32 levels, any single string longer than 1 MB, and payloads above the surface's request limit (web: 50 MB file imports per io-formats; MCP: 10 MB). Rejections SHALL use `INVALID_INPUT` or `LIMIT_EXCEEDED` with the offending pointer.

#### Scenario: Overflowing number
- **WHEN** an input contains the number `1e400`
- **THEN** the call returns `INVALID_INPUT` for that field before reaching the core

#### Scenario: Duplicate keys
- **WHEN** an input object contains the key `lat` twice
- **THEN** the call returns `INVALID_INPUT` naming the duplicate key
