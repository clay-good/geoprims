## 1. Server scaffold

- [x] 1.1 Scaffold `mcp/server.mjs` as a zero-dependency stdio JSON-RPC server using the internal `packages/runtime`; verify it starts, opens no sockets (no-listening-socket scenario), and has an empty `dependencies` list
- [ ] 1.1a Build release artifacts into `mcp/dist/` on tags, with the untagged-checkout message; verify the clone-and-run, untagged-checkout, and verify-before-running scenarios on a clean machine image with only Node installed
- [ ] 1.1b Add the golden surface file and MCP Inspector CLI job; verify the surface-drift scenario (done so far: `mcp/surface.json` holds the tools, resources, templates, and prompts, and `mcp/server.test.mjs` fails on any change to them — it caught this session's describe-description edit and had to be regenerated deliberately. Pending: the MCP Inspector CLI job, which needs CI and a network install the zero-dependency repo does not carry locally.)
- [x] 1.2 Implement protocol negotiation for `2026-07-28`, `2025-11-25`, and `2025-06-18`; verify handshakes from recorded clients of each version (older-client scenario) (the server echoes each of the three versions on initialize, answers a client asking for the retired 2024-11-05 with 2025-11-25 rather than refusing it, and lists all three in order from server/discover)
- [x] 1.3 Verify cross-surface equality: run the golden-vector suite through the server and compare bytes to the website's results (same-result-as-website scenario)

## 2. Meta-tools

- [x] 2.1 Implement `geoprims_search` using the shared core ranker (weighted fields and aliases, `search` module); verify top-3 accuracy on the search fixture set
- [x] 2.2 Implement `geoprims_describe` with `summary`/`schema`/`examples` detail levels and the 20-id limit; verify output sizes per level
- [x] 2.3 Implement `geoprims_run` with schema validation, unit handling, pagination, and summaries; verify the search-then-run and large-polyfill scenarios
- [x] 2.4 Implement `geoprims_pipeline` with binding type checks, unit insertion, and cycle and forward-reference rejection; verify a 4-step chain and a cyclic chain
- [x] 2.5 Implement `geoprims_convert_units`; verify against unit-registry vectors
- [x] 2.5a Add `prefill` to search, citations and limitations to describe, and `summary`, references, `explain` trace, and example-default to run; verify parity with the web page for 20 hero tools (prefill in search; citations, the constants a tool assumes, and the simplified-method limitation in describe; summary, `meta.references`, the example default, and the explain trace in run. `tools/mcp/parity.test.mjs` sweeps every tool on the hero checklist and fails when the page and the server disagree on the sentence, the answer the card shows, the citations, or a single line of the work — and asserts how much it compared, so it cannot pass by comparing nothing.)
- [x] 2.6 Add titles, annotations, output schemas, and structured content plus text blocks; verify the annotations scenario with a conformance checker
- [x] 2.7 Enforce the ≤ 6,000-token default `tools/list` budget in CI; verify with the 4-characters-per-token approximation

## 3. Toolsets, errors, resources, prompts

- [x] 3.1 Implement toolsets (`geodesy-core`, `navigation`, `e6b`, `atmosphere`, `drone-mapping`, `survey-cogo`, `indexing`) and `--no-meta`; verify the e6b and unknown-toolset scenarios
- [x] 3.2 Implement recoverable `isError` results and closest-id suggestions; verify the wrong-id scenario
- [x] 3.3 Implement resources `geoprims://catalog` and `geoprims://tool/{id}` with `ttlMs`/`cacheScope`; verify the resource-read scenario
- [x] 3.4 Implement the five workflow prompts; verify each prompt's tool chain runs end to end
- [x] 3.5 Put model, epoch, accuracy, and not-for-navigation caveats in result `meta` for operational domains; verify the magnetic-caveat scenario

## 4. Limits, assets, and security

- [x] 4.1 Implement timeouts, request-size limits, trap survival, and stderr-only logging without argument values; verify the timeout scenario and a trapping fixture
- [ ] 4.2 Bundle the small offline assets (≤ 6 MB) and implement `--allow-asset-download` with verified, cached tiles; verify the geoid-tile-missing and download-allowed scenarios (done: the bundle. The EGM96 geoid grid and the NADCON5 grids ship in `mcp/dist/assets` at 3.7 MB, WMM2025 and IGRF-14 are compiled into the core, and a gate holds the total under the 6 MB cap and fails if the registry lists a file the bundle does not carry. The offline half of the requirement is already proved by the network audit, which runs every tool's worked example inside a sandbox with no network. Pending: `--allow-asset-download` and both scenarios, which need the EGM2008-1 tiles and an asset origin to download them from; the flag is accepted today and does nothing, because nothing is downloadable yet)
- [x] 4.3 Run the network-sandbox audit (no allowed hosts); verify all offline-capable tools succeed and no connection attempts occur

## 5. Distribution

- [x] 5.1 ~~Configure npm trusted publishing~~ Not done: owner decision 2026-09-23, the server ships in the repository only; `mcp/package.json` is `private`
- [ ] 5.2 Build the MCPB bundle (manifest v0.3, `server.type: node`, toolsets and asset-download option as `user_config`); verify it installs and lists tools in a desktop host (built: `mcp/manifest.json` against the published v0.3 spec, with the six meta-tools, the toolsets setting wired to the server's own `--toolsets`, and node >= 22; `npm run build:mcpb` writes a byte-reproducible zip of the server and its whole dist, 342 files and 6.8 MB, with no dependencies — fixed entry order and timestamps, so the same source gives the same digest. Tests check the manifest describes the server that exists (entry point in the bundle, every advertised tool a real meta-tool, every toolset offered), that the bundle unpacks back to the same bytes through its central directory, and that two builds agree. Pending: installing it in a desktop host, and the asset-download option, which waits on there being downloadable assets)
- [x] 5.3 ~~Write `server.json` for the MCP Registry~~ Not done: no registry listing (owner decision 2026-09-23); `mcp/server.json` removed
- [ ] 5.4 Attach build provenance attestations and SHA-256 digests to release assets; verify attestation verification succeeds
- [x] 5.4a Write README and site setup snippets for Claude Code, Claude Desktop, VS Code (`servers` key), Cursor, and Windsurf for the clone path (the npx path was dropped with npm on 2026-09-23); verify the VS Code key scenario and CLI-testable snippets in CI
- [x] 5.5 Write the website's "Use with agents" page (setup snippets for major MCP clients, toolsets, offline assets); verify every snippet in CI smoke tests

## 6. Agent evaluation

- [x] 6.1 Author 100+ evaluation tasks across all domains with expected ids and tolerances; verify each expected answer against the core (637 tasks across 9 domains, generated from every tool's own worked example: the question a practitioner would type, the tool it means, and the arguments that tool is run with. Each expected answer is the core's own result for that example, so a task cannot drift from the tool it tests)
- [ ] 6.2 Build the evaluation runner and per-release report (selection accuracy, answer accuracy, tokens per task); verify the report publishes with the release (done: `npm run eval:mcp` runs every task through the server's own handlers and reports selection accuracy at 1 and 3, the share answered end to end, and the tokens a round trip costs. Today: 95.4% top-1, 99.8% top-3, 94.8% answered, 406 tokens per task, recorded in data/mcp-eval.json. Pending: publishing the report with a release, which waits on the release pipeline)
- [x] 6.3 Add the 3-point regression gate; verify it blocks a release candidate with degraded descriptions (tools/mcp/eval.test.mjs fails when selection or answer accuracy falls more than 3 points below the recorded baseline, or when a task costs more than 25% more tokens, and it also fails when accuracy rises by more than 3 points without the baseline being updated, so an improvement is recorded rather than quietly widening the allowance)
