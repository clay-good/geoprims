## 1. Server scaffold

- [x] 1.1 Scaffold `mcp/server.mjs` as a zero-dependency stdio JSON-RPC server using the internal `packages/runtime`; verify it starts, opens no sockets (no-listening-socket scenario), and has an empty `dependencies` list
- [ ] 1.1a Build release artifacts into `mcp/dist/` on tags, with the untagged-checkout message; verify the clone-and-run, untagged-checkout, and verify-before-running scenarios on a clean machine image with only Node installed
- [ ] 1.1b Add the golden surface file and MCP Inspector CLI job; verify the surface-drift scenario
- [x] 1.2 Implement protocol negotiation for `2026-07-28`, `2025-11-25`, and `2025-06-18`; verify handshakes from recorded clients of each version (older-client scenario) (the server echoes each of the three versions on initialize, answers a client asking for the retired 2024-11-05 with 2025-11-25 rather than refusing it, and lists all three in order from server/discover)
- [x] 1.3 Verify cross-surface equality: run the golden-vector suite through the server and compare bytes to the website's results (same-result-as-website scenario)

## 2. Meta-tools

- [x] 2.1 Implement `geoprims_search` using the shared core ranker (weighted fields and aliases, `search` module); verify top-3 accuracy on the search fixture set
- [x] 2.2 Implement `geoprims_describe` with `summary`/`schema`/`examples` detail levels and the 20-id limit; verify output sizes per level
- [x] 2.3 Implement `geoprims_run` with schema validation, unit handling, pagination, and summaries; verify the search-then-run and large-polyfill scenarios
- [x] 2.4 Implement `geoprims_pipeline` with binding type checks, unit insertion, and cycle and forward-reference rejection; verify a 4-step chain and a cyclic chain
- [x] 2.5 Implement `geoprims_convert_units`; verify against unit-registry vectors
- [ ] 2.5a Add `prefill` to search, citations and limitations to describe, and `summary`, references, `explain` trace, and example-default to run; verify parity with the web page for 20 hero tools (done: prefill in search, citations and the simplified-method limitation in describe, and summary, the example default, and the explain trace in run, with the default run checked against the primary example for every tool id; pending: the hero-tool parity sweep)
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
- [ ] 4.2 Bundle the small offline assets (≤ 6 MB) and implement `--allow-asset-download` with verified, cached tiles; verify the geoid-tile-missing and download-allowed scenarios
- [x] 4.3 Run the network-sandbox audit (no allowed hosts); verify all offline-capable tools succeed and no connection attempts occur

## 5. Distribution

- [ ] 5.1 Configure npm trusted publishing with provenance for `@geoprims/mcp`; verify provenance on a pre-release and the npx-install scenario
- [ ] 5.2 Build the MCPB bundle (manifest v0.3, `server.type: node`, toolsets and asset-download option as `user_config`); verify it installs and lists tools in a desktop host
- [ ] 5.3 Write `server.json` for `com.geoprims/mcp` with DNS verification, npm and MCPB packages, and `fileSha256`; verify with the registry publisher's validation
- [ ] 5.4 Attach build provenance attestations and SHA-256 digests to release assets; verify attestation verification succeeds
- [x] 5.4a Write README and site setup snippets for Claude Code, Claude Desktop, VS Code (`servers` key), Cursor, and Windsurf for both clone and npx paths; verify the VS Code key scenario and CLI-testable snippets in CI
- [x] 5.5 Write the website's "Use with agents" page (setup snippets for major MCP clients, toolsets, offline assets); verify every snippet in CI smoke tests

## 6. Agent evaluation

- [ ] 6.1 Author 100+ evaluation tasks across all domains with expected ids and tolerances; verify each expected answer against the core
- [ ] 6.2 Build the evaluation runner and per-release report (selection accuracy, answer accuracy, tokens per task); verify the report publishes with the release
- [ ] 6.3 Add the 3-point regression gate; verify it blocks a release candidate with degraded descriptions
