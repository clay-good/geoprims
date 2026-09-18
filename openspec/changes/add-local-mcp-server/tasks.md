## 1. Server scaffold

- [ ] 1.1 Scaffold `packages/mcp` on the MCP TypeScript SDK with stdio transport, depending only on the internal `packages/runtime`; verify it starts and opens no sockets (no-listening-socket scenario)
- [ ] 1.2 Implement protocol negotiation for `2026-07-28`, `2025-11-25`, and `2025-06-18`; verify handshakes from recorded clients of each version (older-client scenario)
- [ ] 1.3 Verify cross-surface equality: run the golden-vector suite through the server and compare bytes to the website's results (same-result-as-website scenario)

## 2. Meta-tools

- [ ] 2.1 Implement `geoprims_search` using the shared BM25 and alias index; verify top-3 accuracy on the search fixture set
- [ ] 2.2 Implement `geoprims_describe` with `summary`/`schema`/`examples` detail levels and the 20-id limit; verify output sizes per level
- [ ] 2.3 Implement `geoprims_run` with schema validation, unit handling, pagination, and summaries; verify the search-then-run and large-polyfill scenarios
- [ ] 2.4 Implement `geoprims_pipeline` with binding type checks, unit insertion, and cycle and forward-reference rejection; verify a 4-step chain and a cyclic chain
- [ ] 2.5 Implement `geoprims_convert_units`; verify against unit-registry vectors
- [ ] 2.6 Add titles, annotations, output schemas, and structured content plus text blocks; verify the annotations scenario with a conformance checker
- [ ] 2.7 Enforce the ≤ 6,000-token default `tools/list` budget in CI; verify with the 4-characters-per-token approximation

## 3. Toolsets, errors, resources, prompts

- [ ] 3.1 Implement toolsets (`geodesy-core`, `navigation`, `e6b`, `atmosphere`, `drone-mapping`, `survey-cogo`, `indexing`) and `--no-meta`; verify the e6b and unknown-toolset scenarios
- [ ] 3.2 Implement recoverable `isError` results and closest-id suggestions; verify the wrong-id scenario
- [ ] 3.3 Implement resources `geoprims://catalog` and `geoprims://tool/{id}` with `ttlMs`/`cacheScope`; verify the resource-read scenario
- [ ] 3.4 Implement the five workflow prompts; verify each prompt's tool chain runs end to end
- [ ] 3.5 Put model, epoch, accuracy, and not-for-navigation caveats in result `meta` for operational domains; verify the magnetic-caveat scenario

## 4. Limits, assets, and security

- [ ] 4.1 Implement timeouts, request-size limits, trap survival, and stderr-only logging without argument values; verify the timeout scenario and a trapping fixture
- [ ] 4.2 Bundle the small offline assets (≤ 6 MB) and implement `--allow-asset-download` with verified, cached tiles; verify the geoid-tile-missing and download-allowed scenarios
- [ ] 4.3 Run the network-sandbox audit (no allowed hosts); verify all offline-capable tools succeed and no connection attempts occur

## 5. Distribution

- [ ] 5.1 Configure npm trusted publishing with provenance for `@geoprims/mcp`; verify provenance on a pre-release and the npx-install scenario
- [ ] 5.2 Build the MCPB bundle (manifest v0.3, `server.type: node`, toolsets and asset-download option as `user_config`); verify it installs and lists tools in a desktop host
- [ ] 5.3 Write `server.json` for `com.geoprims/mcp` with DNS verification, npm and MCPB packages, and `fileSha256`; verify with the registry publisher's validation
- [ ] 5.4 Attach build provenance attestations and SHA-256 digests to release assets; verify attestation verification succeeds
- [ ] 5.5 Write the website's "Use with agents" page (setup snippets for major MCP clients, toolsets, offline assets); verify every snippet in CI smoke tests

## 6. Agent evaluation

- [ ] 6.1 Author 100+ evaluation tasks across all domains with expected ids and tolerances; verify each expected answer against the core
- [ ] 6.2 Build the evaluation runner and per-release report (selection accuracy, answer accuracy, tokens per task); verify the report publishes with the release
- [ ] 6.3 Add the 3-point regression gate; verify it blocks a release candidate with degraded descriptions
