## Context

Motivation is in `proposal.md`. Research is in `docs/research/04-mcp-competitors-legal-a11y.md`. Facts that shape the design (verified 2026-09-18):

- **MCP spec `2026-07-28` is stateless.** It removed the initialize handshake and session ids and added `server/discover`, multi-round-trip requests, and `ttlMs`/`cacheScope` on list results. Tasks moved to an extension. Roots, Sampling, and Logging are deprecated.
- **TypeScript SDK.** v2 is split into packages (`@modelcontextprotocol/server`, `/node`, and others at 2.0.0). The monolithic `@modelcontextprotocol/sdk` 1.30.x is still maintained.
- **Many tools cost context and accuracy.** Anthropic measured 58 tools at about 55K tokens. Tool search with detail levels is the published pattern for large catalogs and raised selection accuracy substantially in Anthropic's evaluation.
- **Client caps.** VS Code allows 128 tools per request; Cursor's cap is reportedly 40.
- **Registry and bundles.** The MCP Registry is in preview (`server.json` schema `2025-12-11`). MCPB manifest is v0.3.
- **npm.** Trusted publishing (OIDC) with provenance is the norm.

## Goals / Non-Goals

**Goals:**
- An agent can find, understand, and correctly call any of about 830 tools in 2–3 tool calls with under 6K tokens of fixed tool schemas.
- Zero network by default. Results are byte-identical to the website.

**Non-Goals:**
- Any second agent surface (CLI, library, WebMCP, remote server).
- MCP Apps (server-provided UI rendered in the host). It is a candidate follow-up to show the map canvas inside agent hosts.

## Decisions

### A1. Two surfaces only: website and MCP server
The website and the MCP server are the whole product. Both load Wasm through one internal, unpublished `packages/runtime` module (loader, validation, asset providers), so there is one code path.

| Alternative | Why not |
|---|---|
| CLI | Duplicates `geoprims_run`/`geoprims_pipeline` for scripting and the website's batch mode for CSV. Adds a versioned public interface (flags, exit codes) to support forever. |
| Public npm library | A public API with semver obligations, typed bindings, and multi-runtime support, for an audience the MCP server and website already serve. Can be added later without changing either surface, because `packages/runtime` already exists. |
| WebMCP | Experimental browser API in origin trial; a third surface. |

### A2. Meta-tools by default, direct toolsets opt-in
Six meta-tools (`search`, `describe`, `run`, `pipeline`, `convert_units`, `report_problem`) are the default surface. Curated toolsets of ≤ 40 tools serve clients with native deferred tool search, or users who want first-class tools for one workflow.

| Alternative | Why not |
|---|---|
| Expose all ~830 tools | ~120K–320K tokens; exceeds client caps; lower selection accuracy. |
| Grow the tool list dynamically with `list_changed` | Uneven client support; breaks prompt caching; the stateless spec needs a `subscriptions/listen` stream. |
| One `run` tool with free-text arguments | Loses schema validation and discoverability. |

`geoprims_search` uses the single core ranker (weighted fields, aliases, prefix stemming, one-edit typo tolerance, quantity extraction), the same one the web palette uses, so agent and human search rank identically. If the SEP-1821 `tools/list` query filter lands in a future spec, `search` maps onto it.

### A3. Runtime: Node.js, zero dependencies, clone-and-run
Node is bundled with major desktop hosts. The server is a zero-dependency ES module (`mcp/server.mjs`). It implements the stdio JSON-RPC subset MCP needs: newline-delimited framing, version negotiation across `2026-07-28`, `2025-11-25`, and `2025-06-18`, and a message size cap. This follows roughlogic.com's `mcp/server.mjs`, which runs from a clone with `node` alone (`docs/research/06` §6).

Why not the official SDK:
- A clone must run with no `npm install`, and the server must be auditable by reading one directory.
- No dependency supply chain for a tool that promises no network access.

Conformance comes from the MCP Inspector CLI in CI, plus recorded handshakes from real clients.

Release tags carry CI-built artifacts (`mcp/dist/`: Wasm modules, catalog, bundled assets), so clone users need no Rust toolchain. The npm package and MCPB bundle are built from the same tag.

Fallback: if protocol churn makes the hand-rolled layer costly, adopt the SDK behind the same surface. The golden surface file keeps it identical.

Also rejected: a single compiled binary (50–100 MB per platform, code-signing burden) and a Wasm component host (no mainstream MCP host supports one).

### A4. Pipeline binding syntax
`bind` maps a target input JSON Pointer to `"<stepIndex>:<outputPointer>"`. The server type-checks each binding: the quantity type and unit must be compatible, or a unit conversion is inserted. It rejects cycles and forward references. This mirrors web tool chaining, so an agent can express "MGRS → lat/lon → geoid height → orthometric height" in one call.

### A5. Pagination and summaries for large outputs
Large collections are capped per call, with `total`, `offset`, and `truncated`, plus summaries (count, bounding box, area). Agents rarely need 250,000 H3 cells in context. They need the count, the extent, and a way to page.

### A6. Assets for agents
- **Bundled:** the small assets (WMM2025, WMMHR2025, IGRF-14, EGM96-15, the CRS registry, Natural Earth 110m, deformation zones), at most 6 MB.
- **Large packs:** fetched tile-by-tile only when the server is launched with `--allow-asset-download`. Tiles are the same verified, coarse tiles as the website, cached under the OS cache directory.
- **Default:** fully offline, which keeps the "no network" promise literal.

### A7. Agent evaluation as a release gate
A fixed set of 100+ tasks, each with an expected tool id and answer tolerance, runs against at least one current frontier model per release. It measures selection accuracy, answer accuracy, and tokens per task. Descriptions and aliases are product surface and are tuned against this eval, not by guesswork.

## Risks / Trade-offs

- **[MCP spec churn (the stateless model is new; clients lag)]** → Support three protocol versions through the hand-rolled negotiation layer. Run the conformance suite against recorded client handshakes.
- **[MCP Registry is preview; data may reset]** → npm and MCPB work without it.
- **[Meta-tool indirection costs an extra round trip]** → `describe` accepts up to 20 ids. `search` summaries often suffice to run simple tools. `pipeline` removes round trips for chains.
- **[Agents drop caveats]** → Caveats live in `meta` (model, epoch, accuracy, not-for-navigation), which travels with every result.
- **[Users who want scripting miss a CLI]** → Document calling the MCP server from scripts via any MCP client library. Revisit a CLI only on demonstrated demand.

## Migration Plan

Not applicable (greenfield). Release order: MCP server on npm → MCPB bundle → registry entry.

## Open Questions

None that affect the spec.
