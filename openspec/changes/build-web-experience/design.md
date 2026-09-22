## Context

Motivation is in `proposal.md`. This change builds on the manifest, compute core, and asset registry from `establish-platform-foundation`. Research: `docs/research/02-indexing-wasm-rendering-hosting.md` (rendering, PWA, search) and `docs/research/04-mcp-competitors-legal-a11y.md` (accessibility, competitors).

Constraints:
- About 830 tool routes (about 300 of them indexable pages) must be pre-rendered, fast, and readable without JavaScript.
- WebGPU is not universal in September 2026. Firefox on Linux, Intel Mac, and Android, plus many Linux Chrome GPUs, need WebGL2.
- Safari evicts tab storage after 7 days without a visit. Home-screen installs are exempt.
- No third-party requests (privacy spec), so every font, map layer, and library is served from the site itself.
- One static website on Cloudflare plus the local MCP server. There is no map or tile service to run.

## Goals / Non-Goals

**Goals:**
- Forms, docs scaffolds, and search entries generated from manifests. No hand-built page per tool.
- Shell JS ≤ 90 KB compressed; canvas and Wasm load after first paint.
- One renderer abstraction that runs on WebGPU, WebGL2, and a Canvas2D fallback.

**Non-Goals:**
- A general map-authoring UI or tile-server integration.
- Server-side rendering at request time (all pages are built ahead).

## Decisions

### W1. Static site generator: Astro with Svelte 5 islands
Astro renders every endpoint page to static HTML at build time and ships zero JS by default. Interactive parts (form, results, canvas, palette) hydrate as islands. Svelte 5 gives a small runtime.

| Alternative | Why not |
|---|---|
| SvelteKit static adapter | Also viable. Astro's content collections and zero-JS default fit a docs-heavy site of hundreds of pages better. |
| Next.js static export | Larger runtime; React hydration cost on every page. |
| Eleventy + vanilla TS | Lightest, but reimplements the component model for forms and the palette. |

### W2. Schema-driven form engine
One `ToolForm` component renders any manifest input schema: coordinate fields (multi-notation parser from the geodesy core), quantity fields (value plus unit selector), enums, arrays (vertex lists with import), and nested objects. Validation runs twice: a cheap schema check in the UI thread for field messages, then the authoritative check in Wasm. The Wasm result wins on disagreement, and a test asserts they agree on all golden vectors.

### W3. Renderer: luma.gl device abstraction plus custom layers
luma.gl 9.x gives one device API over WebGPU and WebGL2 with no map chrome and no API keys. geoprims writes its own small layer set (the layer kinds in `map-canvas`).

| Alternative | Why not |
|---|---|
| deck.gl | Good layer catalog, but larger. Its geospatial layers assume Web Mercator tiles; we need ellipsoidal densification and a globe. It could back `cell-set` layers later. |
| MapLibre GL JS 6 | WebGL2 only, and built around tiled basemaps, which geoprims does not use. |
| CesiumJS | Full 3D globe, but multiple MB. Overkill for vector overlays. |
| three.js | General-purpose. Projection and globe math would be ours anyway. |

- **Geometry comes from Wasm.** Densification, antimeridian splitting, and pole handling run in the core so the canvas draws exactly the math the tool computed. The renderer only projects to screen.
- **Canvas2D fallback** supports points, lines, polygons, and vector diagrams with no GPU.
- **Style.** Flat cartographic rendering (fills, hairlines, casings) plus one shading pass for the globe's limb. No post-processing effects.
- **Animation.** A scene clock drives the playhead. Each frame asks the core for the state at that time (positions, separation, sun vector), so what plays is exactly what the tool computed. Camera moves use one ease-out curve.

### W4. Basemap: Natural Earth only
- Natural Earth 110m (public domain) is bundled; 50m loads on demand as one static file from the site. Both are drawn as vector land fills, coastlines, borders, lakes, and a graticule in the Atlas style.
- No tiled basemap. A vector-tile basemap (Protomaps PMTiles, OpenStreetMap) was considered and dropped: it would add hosting beyond the static site, ODbL attribution everywhere, and tile requests that hint at location. The tools' own geometry is the content; the base layer only gives it context.
- Fetching the whole 50m file (never a region of it) means map use reveals nothing about location.

### W5. Command palette search: the core ranker
Ranking uses the single deterministic ranker compiled into the core (per `discovery/natural-language-prefill`): weighted fields (title, id, aliases, keywords, group), prefix stemming, one-edit typo tolerance, and boosts for exact alias, prefix, pinned, and recency. The palette calls it on every keystroke (well under 1 ms for ~1,000 entries in Wasm). uFuzzy is used only to compute match highlighting. Ranking quality is tested with the shared query fixture.
- Paste-to-detect runs a detector chain in Wasm (H3, S2 token, geohash, quadkey, XYZ, Maidenhead, MGRS, coordinate notations, altimeter group) and returns all plausible interpretations.

### W6. Permalink encoding
Fragment = `#v1:` + base64url(deflate-raw(canonical JSON of inputs, units, view)). Compression uses the native `CompressionStream`, with a tiny fallback. The version prefix allows migrations; each tool's manifest can declare field renames for fragment migration.

### W7. PWA and caching: Workbox
- Precache: shell with an offline route renderer, catalog, search index, and Wasm (≤ 12 MB compressed), excluding the report dialog module. Docs pages are cached when visited or via an optional Docs pack.
- Runtime cache-first for immutable, content-hashed assets.
- Offline packs go to OPFS or the Cache API with integrity checks (data-assets spec).
- Updates use a "waiting" service worker and a user prompt.

### W8. Build-time math: Temml (LaTeX → MathML)
Formulas are authored in LaTeX in docs, rendered to MathML at build, and checked in the accessibility job. No runtime math library.

### W9. Visual design: Atlas
Minimal and modern, like the best product tools, and deliberately unlike the sister sites (roughlogic, sophiewell), which are near-black with system fonts and a blue or white accent.

| Token | `paper` | `ink` |
|---|---|---|
| Background | `#F7F6F3` warm off-white | `#0E1015` blue-graphite |
| Raised surface | `#FFFFFF` | `#161920` |
| Text | `#15171C` (16.6:1) | `#ECEEF2` (16.4:1) |
| Muted text | `#5B616E` (5.8:1) | `#9AA1AE` (6.8:1) |
| Signal accent | `#B53C0A` (5.3:1 on the page; 4.6:1 through deuteranopia, darkened from `#C2410C` by the 2.5 audit) | `#FF8A4C` (8.2:1) |

- Type: Geist Sans for prose and Geist Mono for numbers (both SIL OFL 1.1, self-hosted, subset). Big answer numerals, small muted units.
- Layout: generous whitespace, hairline dividers, 8 px radius, almost no shadow. The canvas gets the most space on the page.
- Delight comes from the canvas (smooth camera, paths drawing in, scenes you can play and scrub), not from decoration on the chrome.
- Exact values are a starting point for task 2.1. The spec fixes the structure (one accent, neutral surfaces, AA contrast), not the hex codes.

### W10. Audio: raw Web Audio API
A ~5 KB module synthesizes the fixed sound set with oscillators, noise buffers, and gain envelopes. It is lazy-loaded only when audio is enabled.

### W11. Testing stack
- End-to-end and cross-browser: Playwright (Chromium, Firefox, WebKit).
- Accessibility: axe-core, run in every theme mode.
- Performance: Lighthouse CI budgets on a sampled route set.
- Visual regression: canvas and components, screenshot diffs per mode.
- Egress privacy: proxy capture (from the foundation change).

## Risks / Trade-offs

- **[luma.gl and deck.gl still call WebGPU experimental; API churn]** → Keep the renderer behind our own thin interface. Pin versions. WebGL2 is the tested baseline.
- **[Hundreds of pages × previews makes build time long]** → Incremental builds keyed on manifest hash. Preview images come from the headless Canvas2D renderer in parallel.
- **[Map fills reduce label and line contrast]** → Contrast is measured on rendered pixels over the real fills (visual-theme spec), and lines and labels carry a casing.
- **[Natural Earth is coarse at street scale]** → The tools' own geometry stays exact at every zoom; the readout says when the base layer is generalized.
- **[Permalinks leak inputs when users paste them into chat or email]** → The fragment keeps them off our servers, not away from recipients. The share dialog says so.
- **[Safari storage eviction breaks offline packs]** → Prompt to install. Show the eviction warning.

## Migration Plan

Not applicable (greenfield). The site launches with the tools that reach `stable` in the domain changes; experimental tools are visible but labeled.

## Open Questions

- None for the visual direction; exact token values are tuned in task 2.1.
