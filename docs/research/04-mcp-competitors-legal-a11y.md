# Research: MCP, agent distribution, competitors, legal, and accessibility

> Research brief gathered 2026-09-18 to inform the OpenSpec changes. Flagged items still need checking against a primary source.


## 1. MCP specification: current state

**Latest version: `2026-07-28`**, released July 28, 2026. Earlier dated versions were 2025-06-18 and 2025-11-25. Spec: https://modelcontextprotocol.io/specification/2026-07-28 · release notes: https://blog.modelcontextprotocol.io/posts/2026-07-28/

Changes in 2026-07-28 that affect geoprims:
- **The protocol no longer keeps sessions.** The `initialize`/`initialized` handshake and the `Mcp-Session-Id` header are gone. Every request carries its protocol version, client info and capabilities in `_meta`. A new optional `server/discover` call returns capabilities up front. This suits a pure-compute server: each call stands alone, so any state has to go in explicit handles passed as arguments.
- **Multi Round-Trip Requests (MRTR).** A server can return `resultType: "input_required"` along with an elicitation request. The client retries the call with `inputResponses`. This replaces the old model where the server sent requests of its own.
- **List results can be cached.** `tools/list`, `prompts/list`, `resources/list` and `resources/read` now carry `ttlMs` and `cacheScope`. The spec says tool order SHOULD be deterministic "to improve prompt cache hit rates."
- **Tasks is now an extension.** It moved out of core into `io.modelcontextprotocol/tasks` (poll with `tasks/get`, plus new `tasks/update`). Other official extensions: **MCP Apps** (the server ships HTML UI that the host renders in a sandboxed iframe) and Skills over MCP.
- **Deprecated, with at least 12 months' notice:** Roots, Sampling, Logging, and the legacy HTTP+SSE transport.
- **Transports:** stdio and Streamable HTTP. HTTP now carries the method and tool name in `Mcp-Method` / `Mcp-Name` headers, and tools can mark parameters with `x-mcp-header`.

**Tool definition fields** (https://modelcontextprotocol.io/specification/2026-07-28/server/tools):
- Fields: `name`, `title`, `description`, `icons`, `inputSchema`, optional `outputSchema`, `annotations`.
- Behavior hints go in `annotations`: `readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`. Clients MUST treat these as untrusted.
- Results can include `structuredContent`, which must match `outputSchema` when one is given. Servers SHOULD also include the same JSON as a text block.
- Tool names SHOULD be 1–128 characters from `[A-Za-z0-9_.-]`, so dotted names like `geo.inverse` are fine.
- **JSON Schema dialect:** 2020-12 unless `$schema` says otherwise. For a tool with no inputs, the spec recommends `{"type":"object","additionalProperties":false}`.
- Change notification is now `notifications/tools/list_changed`, delivered over a `subscriptions/listen` stream.
- Errors: input validation problems go back as `isError: true` tool results so the model can correct itself. Unknown tools are JSON-RPC errors.

**TypeScript SDK (checked on npm):**
- **v2 is split into packages:** `@modelcontextprotocol/server`, `/client`, `/core`, `/node`, `/express`, `/hono` and `/ext-apps`, all at **2.0.0** (published July 27, 2026). It depends on `zod ^4.2.0`.
- The single-package `@modelcontextprotocol/sdk` is still maintained at **1.30.0** (updated September 17, 2026).
- The Python, Go and C# SDKs supported 2026-07-28 on release day. The Rust SDK support is in beta.

**MCP Registry** (https://modelcontextprotocol.io/registry/about):
- Status: **still in preview.** Breaking changes and data resets are possible, and there is no GA date I could find.
- `server.json` schema version: `2025-12-11`.
- Server names use reverse DNS. `io.github.<user>/…` is verified through GitHub; `com.geoprims/…` would be verified through a DNS or HTTP challenge.
- Package types and how ownership is checked (https://modelcontextprotocol.io/registry/package-types):

| Type | Ownership check |
|---|---|
| npm | `"mcpName"` field in `package.json` |
| PyPI, NuGet | `mcp-name:` string in the README |
| cargo | visible `mcp-name:` text (crates.io strips HTML comments) |
| OCI | `io.modelcontextprotocol.server.name` label |
| mcpb | GitHub or GitLab release URL plus `fileSha256`; clients check the hash |

- The registry is meant to feed aggregators, not host apps directly.

**MCPB (formerly DXT):**
- Manifest spec **v0.3** (https://github.com/modelcontextprotocol/mcpb/blob/main/MANIFEST.md). Server types: `node`, `python`, `binary`, `uv`.
- Manifests declare `tools` statically, and `tools_generated: true` for tools created at runtime. `compatibility.platforms` covers darwin, win32 and linux.
- CLI: `@anthropic-ai/mcpb` 2.1.2 (`mcpb init` / `mcpb pack`).
- Anthropic recommends Node because Claude Desktop ships with it. MCPB runs over stdio, works offline and needs no OAuth. Anthropic now calls MCPB the "secondary" path and prefers remote servers for directory listing.
- Connectors Directory submission requires **annotations on every tool** and a privacy policy (https://claude.com/docs/connectors/building/mcpb).

**Security for a local server:**
- Prefer **stdio**. The DNS-rebinding advisories only affect HTTP servers.
- If geoprims ever offers localhost HTTP: validate the `Origin` and `Host` headers, bind to 127.0.0.1 only, and require a per-launch token. The TypeScript SDK shipped DNS-rebinding protection off by default until 1.24.0 (GHSA-w48q-cv73-mx4w). Best practices: https://modelcontextprotocol.io/docs/2026-07-28/tutorials/security/security_best_practices
- For a pure-math Wasm server:
  - Mark every tool `readOnlyHint: true, idempotentHint: true, openWorldHint: false`.
  - Make no network calls and do no filesystem writes.
  - Cap input sizes (for example, polygon vertex counts) and set per-call timeouts to prevent denial of service.
  - Treat descriptions as a prompt-injection surface: keep them static and don't echo user strings into them.

## 2. Exposing ~800 tools to an LLM

**Cost and accuracy:**
- In Anthropic's own measurement, 58 tools across 5 servers used about 55K tokens before any work began. At a rough 150–400 tokens per tool, 800 full schemas would be about 120K–320K tokens. That is my estimate, not a measurement, but it would exceed or nearly fill most context windows.
- Anthropic's Tool Search Tool (November 24, 2025) raised MCP-eval accuracy from 49% to 74% on Opus 4 and from 79.5% to 88.1% on Opus 4.5, and cut tokens about 85%. It uses `defer_loading: true` and comes in regex and BM25 variants. Anthropic suggests it when there are more than 10 tools or more than 10K tokens of definitions. Adding 1–5 tool use examples raised accuracy on complex parameters from 72% to 90% (https://www.anthropic.com/engineering/advanced-tool-use).
- "Code execution with MCP" (November 4, 2025) showed 150K tokens dropping to 2K. It recommends a `search_tools` call with a detail-level parameter (name only, name plus description, or full schema), and tools exposed as files in a code tree (https://www.anthropic.com/engineering/code-execution-with-mcp).

**Hard limits in clients:**
- **VS Code Copilot: 128 tools per request.** It offers "virtual tools" to work around this (microsoft/vscode#290356).
- **Cursor: reported cap of 40 tools** (https://mcpverdict.com/mcp/clients/cursor/). This comes from a third-party site and may have changed; verify before relying on it.
- Claude Code defers MCP tools automatically through `ENABLE_TOOL_SEARCH` (auto at 10% of context). There are open bug reports about it not triggering and about HTTP servers not being deferred (anthropics/claude-code#19890, #40314).

**Where the spec is heading:**
- SEP-1821 proposes a `query` parameter on `tools/list` plus a `tools.filtering` capability. It has **not landed in 2026-07-28** (https://github.com/modelcontextprotocol/modelcontextprotocol/issues/1821).
- The August 22, 2026 roadmap names "progressive discovery" of large catalogs as a priority, but gives no date (https://blog.modelcontextprotocol.io/posts/mcp-roadmap/).

**Recommendation:** expose a small fixed set of meta-tools by default, and let power users turn on a curated set of direct tools (see the final section). Don't depend on `list_changed` to grow the tool set during a session. Client support for it is uneven, it breaks prompt caching, and the stateless model requires a `subscriptions/listen` stream to receive it.

## 3. Browser-side MCP / WebMCP

- **What it is:** a spec draft in the W3C **Web Machine Learning Community Group**. It is a Community Group draft, not a W3C standard (https://github.com/webmachinelearning/webmcp).
- **API:** `document.modelContext`, migrating from `navigator.modelContext`, so detect both. Methods: `registerTool()` (name, description, inputSchema, callback; unregister by aborting its AbortSignal), `getTools()`, `executeTool()`, and a `toolchange` event. There is a declarative option that turns `<form>` elements into tools. Work in progress includes service-worker registration and output-schema validation.
- **Status:** Chrome 146 Canary had it behind a flag (February 2026). An **origin trial runs from Chrome 149 through Chrome 156**, announced June 9, 2026 (https://developer.chrome.com/blog/ai-webmcp-origin-trial). For local testing, use `chrome://flags/#enable-webmcp-testing`. I found no confirmed Firefox or Safari support. Edge support is claimed by a third-party source only.
- **Fit for geoprims:** cheap to add, because the same Wasm functions and tool registry could be registered on each tool page. But tools can only be found by visiting the page and the design assumes a human in the loop, so it **complements the local MCP server and doesn't replace it.** `@mcp-b/global` 5.1.0 is a polyfill if you want to support it before browsers ship it.

## 4. Competitor and adjacent landscape

I checked ads and trackers by fetching each page's HTML (see notes after the table).

| Product | Model | What it lacks |
|---|---|---|
| Movable Type latlong | Free, client-side JS, reference-quality | No ad networks found; loads the Google Maps API (a third-party request with a key). Covers about a dozen functions, has no API, and has no agent access |
| NGS NCAT / VDatum / NADCON 5 | Free US government services; NCAT has a JSON **API** (https://geodesy.noaa.gov/web_services/ncat/) | Server-side; covers only US territories; online only; does not convert between orthometric and ellipsoidal heights |
| NOAA/CIRES magnetic calculators | Free; WMM2025 is valid through late 2029 | Server-side; the web service needs **API registration** (https://www.ngdc.noaa.gov/geomag/calculators/magcalc.shtml) |
| CalcMaps | Map tools | **17 adsbygoogle hits** plus Google Tag Manager/gtag; server-hosted |
| epsg.io (MapTiler) | CRS lookup and transform | **AdSense plus Google Analytics** |
| MyGeodata Converter | File conversion | Files are **uploaded to a server**; freemium limits; gtag |
| Omni Calculator | Thousands of calculators | **Playwire ad stack**, logins, Google Tag Manager; no geodesy depth |
| Pix4D GSD calculator | One calculator | Marketing funnel with Google Tag Manager |
| DroneDeploy | Enterprise SaaS | Accounts and a subscription; its calculators are not standalone |
| e6bx.com | SPA with aviation calculators | Web version is free; no ad network found in the main bundle; **mobile app is paid ($8.99 iOS)**; no API |
| Sporty's E6B app, ForeFlight | Paid apps (ForeFlight by subscription) | Closed and mobile-only; no agent access. I did not verify their current pricing |
| h3geo.org | Docs and viewer | H3 only; h3-js 4.5.0 |
| GeographicLib online | CGI **server-side**, sourceforge (https://geographiclib.sourceforge.io/cgi-bin/GeodSolve) | Dated UI; the library is excellent (MIT) and should be the accuracy reference |
| CyberChef (GCHQ) | UX analogue: hundreds of chainable client-side operations, Apache-2.0 | Not geospatial; its "recipe" chaining is worth borrowing |
| it-tools.tech | UX analogue: developer tools collection, client-side, open source | Not geospatial |

Notes on the table:
- The MyGeodata upload and the Omni login were inferred from page markup and known product behavior.
- Only these pages were scanned: CalcMaps, epsg.io, Omni, MyGeodata, Pix4D, e6bx, Movable Type, it-tools, GeographicLib and h3geo.org. The Omni ad stack was detected by the "playwire" string. Scripts injected at runtime could be missed.
- **Common gaps across the field:** ads or trackers, uploads or server compute, no offline mode, no API, and **no agent/MCP access**. None of them covers geodesy, aviation, drones and surveying in one consistent interface.

## 5. Existing geospatial and aviation MCP servers

| Server | Scope | Gaps |
|---|---|---|
| **gis-mcp** (mahdin75) | Python; Shapely, PyProj, GeoPandas, Rasterio, PySAL; 92+ tools (https://github.com/mahdin75/gis-mcp) | Heavy install; GDAL/PROJ native dependencies |
| **Mapbox MCP** | Mostly API calls needing a Mapbox token, plus offline Turf.js tools (distance, bearing, buffer, point-in-polygon) (https://github.com/mapbox/mcp-server) | Offline part is limited to Turf-level geometry |
| **bbox-mcp-server** | Coordinates, EPSG, H3, map links (https://github.com/iamvibhorsingh/bbox-mcp-server) | Small |
| **Aviation weather** (cyanheads, finack, Perufitlife and others) | Live METAR/TAF/PIREP data fetchers | Data retrieval only, not calculation |

**Official registry search (my queries):** "geodesy" returned 0 results and "e6b" returned 0. "coordinates" returned one generic server. "drone" returned market-intel and governance servers only.

**Gap geoprims fills:** an offline, deterministic, dependency-free calculation engine covering precise geodesy on the ellipsoid, datums, magnetic models, E6B and performance, photogrammetry/GSD, and COGO/traverse. It would work without a network, return structured output checked against schemas, and match the website's results exactly.

## 6. Distribution for a local server

| Option | Pros | Cons |
|---|---|---|
| **npm + `npx -y @geoprims/mcp`** | Default for Claude Code, Cursor and VS Code; the Wasm file sits inside the package | Needs Node (Claude Desktop ships it) |
| **MCPB (`server.type: node`)** | One-click install in Claude Desktop; registry type `mcpb` via GitHub Releases | Desktop only |
| Bun `--compile` / Deno `compile` single binary | No runtime needed; Wasm can be embedded | Roughly 50–100MB per OS/architecture; macOS notarization and Windows signing issues. My assessment; I did not re-verify it |
| Docker/OCI (ghcr.io) | Enterprise sandboxing | Overkill for pure compute; slow cold start |
| Python `uvx` wrapper | Reaches Python users | Second binding to maintain (wasmtime-py) |
| Wasm component (WASI P2) | Ideal long term | **No mainstream MCP host loads Wasm components directly yet.** I found no evidence of one; treat as unverified |

**Supply chain:**
- Publish to npm through **trusted publishing (OIDC) from GitHub Actions**. That generates provenance attestations automatically (GA July 31, 2025).
- Classic npm tokens were **revoked on December 9, 2025**. Granular write tokens now last at most 90 days.
- Bypass-2FA tokens will lose direct-publish rights in January 2027.
- Configurations created after September 3, 2026 default to staged publishing.
- Sources: https://docs.npmjs.com/trusted-publishers/, https://github.blog/changelog/2025-12-09-npm-classic-tokens-revoked-session-based-auth-and-cli-token-management-now-available/, https://github.blog/changelog/2026-07-31-restricting-npm-bypass-2fa-granular-access-tokens/
- Also: attach Sigstore/SLSA attestations to the `.mcpb` and binary release assets (for example with `actions/attest-build-provenance`), publish SHA-256 hashes, and put `fileSha256` in `server.json`.

## 7. Analytics, legal, accessibility

**Analytics:**
- Options: Plausible, Umami, GoatCounter and Cloudflare Web Analytics, all cookieless.
- Consent still applies beyond cookies: EU ePrivacy Article 5(3) covers *any* storage on or access to the device. Umami, for example, uses localStorage. Only purely server-log or edge analytics avoid this completely. Treat that reading as legal nuance; get counsel.
- One comparison source says GoatCounter has no hosted SaaS. That is wrong: goatcounter.com exists.
- The strongest privacy story is **no analytics at all**, or aggregate counts from CDN logs. That also fits "no tracking." The MCP server should report nothing back.

**Aviation and navigation disclaimers:**
- State: "not certified for navigation; not a substitute for approved flight manuals/POH, official charts, NOTAMs, or an FAA-approved E6B/avionics."
- Any aviation output depends on the user checking it. Show the model epoch, such as WMM2025, valid 2025–2029.
- Put a short `accuracy`/`model` note in tool descriptions and structured outputs so agents pass the caveats along.

**Export controls:**
- Under the EAR, software and technology that is **"published"** (freely posted on the internet without restriction) is **not subject to the EAR** at all (15 CFR 734.3(b)(3) and 734.7; https://www.ecfr.gov/current/title-15/subtitle-B/chapter-VII/subchapter-C/part-734/section-734.7).
- General geodesy, E6B, photogrammetry and COGO math is textbook and public-domain-type material, and a free open website fits that rule.
- The ITAR public-domain definition is 22 CFR 120.34.
- **Caveats:** EAR99 applies to items that *are* subject to the EAR, so for a published tool the more accurate statement is "not subject to the EAR." Avoid anything aimed at weapons, such as ballistic trajectories, targeting or fire control, or munitions guidance, which could raise ITAR/USML Cat. XII/IV questions. Also avoid encryption features. Get a short export-counsel review. I am not a lawyer.

**Accessibility:**
- Target **WCAG 2.2 AA**. The US Title II (government) rule is still WCAG 2.1 AA, and DOJ extended its deadlines to April 26, 2027 and April 26, 2028 (https://www.federalregister.gov/documents/2026/04/20/2026-07663/...). The EU Accessibility Act has applied since June 28, 2025, with a microenterprise exemption.
- Contrast ratios I computed on pure black:

| Color | Ratio | Result |
|---|---|---|
| Amber #FFB000 | 11.46 | Passes AAA |
| Phosphor green #33FF33 | 15.49 | Passes |
| #00C832 | 9.31 | Passes |
| Dim green #008F11 | 4.94 | Barely passes AA for body text |
| #1A7A1A | 3.84 | **Fails** for text |
| Red #CC0000 | 3.57 | **Fails** |
| #FF3B30 | 5.92 | Passes |

- The risks are in the "dim" secondary text, gridlines used as meaning (1.4.11 requires 3:1 for non-text elements), scanline or glow overlays that lower effective contrast, and red/green status coding for users with color vision deficiencies.
- WCAG 2.2 additions to handle:
  - 2.4.11 Focus Not Obscured (sticky HUD bars)
  - 2.4.7 visible focus rings
  - 2.5.8 targets at least 24×24 CSS px
  - 2.5.7 alternatives to dragging on map and compass widgets
  - 3.3.7 Redundant Entry
- Also: honor `prefers-reduced-motion` (flicker and scanline animation; 2.3.1 limits flashes to 3 per second), offer a light or high-contrast theme, and use `aria-live` for computed results. Monospace digits help screen-reader and zoom users too.

## Recommended MCP architecture for 800 tools

1. **A Rust or AssemblyScript core compiled to one Wasm module**, plus a generated **tool registry JSON**. Each entry holds id, domain, title, summary, input and output JSON Schema 2020-12, units, a 1–3 item example set, and accuracy and model notes. The website UI, the MCP server and the WebMCP registrations are all generated from this registry.
2. **Default surface (about 5 tools, stable, cacheable, deterministic order):**
   - `geoprims_search(query, domain?, limit)`: BM25 over names, summaries and tags; returns ids and one-line summaries.
   - `geoprims_describe(ids[], detail: "summary"|"schema"|"examples")`: follows Anthropic's detail-level pattern.
   - `geoprims_run(id, args)`: validates against the tool's schema and returns `structuredContent` plus a text block; errors come back as `isError` with corrective hints.
   - `geoprims_batch(calls[])` or `geoprims_pipeline`: CyberChef-style chaining with no model round-trips.
   - `geoprims_units_convert`.
   - All marked `readOnlyHint`/`idempotentHint: true`, `openWorldHint: false`.
3. **Optional direct toolsets**, turned on by a CLI flag or MCPB `user_config`, for example `--toolsets=e6b,geodesy-core`. Keep each set under about 40 tools so it fits Cursor's reported cap and stays well under Copilot's 128. These let clients with native tool search, such as Claude with `defer_loading`, index real tools.
4. **Resources:** `geoprims://catalog` (the index) and `geoprims://tool/{id}` (the full spec plus a worked example). **Prompts:** workflow recipes such as "preflight nav log" and "photogrammetry flight plan."
5. **No state, stdio first.** Include `ttlMs`/`cacheScope: "public"` on list results. Offline only, with no network.
6. **Optional:** an **MCP Apps** extension that renders the HUD widget inline, and **WebMCP** registration on geoprims.com that reuses the same registry.
7. **Ship as:** npm (`npx`, with OIDC provenance), `.mcpb` for Claude Desktop, and a `server.json` under `com.geoprims/mcp` with DNS verification. Later: single binaries and an OCI image.
8. **Plan for SEP-1821.** If `tools/list?query=` lands, `geoprims_search` maps straight onto it.

## Differentiation summary

- **The only large, cross-domain calculation engine** covering geodesy, aviation, drones and surveying. Competitors are single-purpose, US-only, or general "omni" calculators without geodetic depth.
- **Fully client-side, offline, with no accounts, ads or tracking.** Most direct competitors (CalcMaps, epsg.io, Omni, MyGeodata) run ads or Google trackers, or upload your data to a server.
- **Agent-native:** a local MCP server that is **identical to the website bit for bit** (same Wasm), with checked outputs and stated accuracy and model epochs. Existing MCP servers either fetch data (aviation weather), need native GIS dependencies (gis-mcp), or require tokens (Mapbox). No geodesy or E6B calculation server exists in the official registry.
- **Built for context limits:** about 5 meta-tools instead of 800 schemas. Anthropic's published numbers show large accuracy and token gains from this pattern.
- **Trust:** provenance-signed packages, a published accuracy reference (GeographicLib, NGS test vectors), clear aviation disclaimers, and a HUD aesthetic that meets WCAG 2.2 AA.

**Most uncertain points:** Cursor's 40-tool cap is current only per a third-party site. I found no GA date for the MCP Registry. Wasm-component MCP hosting is unverified. Browser support for WebMCP beyond Chrome is unknown. The ad-detection method could miss scripts injected at runtime. The export-control conclusions need a counsel review.
