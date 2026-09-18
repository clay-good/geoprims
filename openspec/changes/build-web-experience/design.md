## Context

Motivation is in `proposal.md`. This change builds on the manifest, compute core, and asset registry from `establish-platform-foundation`. Research: `docs/research/02-indexing-wasm-rendering-hosting.md` (rendering, PWA, search) and `docs/research/04-mcp-competitors-legal-a11y.md` (accessibility, competitors).

Constraints:
- About 800 endpoint pages must be pre-rendered, fast, and readable without JavaScript.
- WebGPU is not universal in September 2026. Firefox on Linux, Intel Mac, and Android, plus many Linux Chrome GPUs, need WebGL2.
- Safari evicts tab storage after 7 days without a visit. Home-screen installs are exempt.
- No third-party requests (privacy spec), so every font, basemap tile, and library is self-hosted.

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
| SvelteKit static adapter | Also viable. Astro's content collections and zero-JS default fit a docs-heavy site of 800 pages better. |
| Next.js static export | Larger runtime; React hydration cost on every page. |
| Eleventy + vanilla TS | Lightest, but reimplements the component model for forms and the palette. |

### W2. Schema-driven form engine
One `ToolForm` component renders any manifest input schema: coordinate fields (multi-notation parser from the geodesy core), quantity fields (value plus unit selector), enums, arrays (vertex lists with import), and nested objects. Validation runs twice: a cheap schema check in the UI thread for field messages, then the authoritative check in Wasm. The Wasm result wins on disagreement, and a test asserts they agree on all golden vectors.

### W3. Renderer: luma.gl device abstraction plus custom layers
luma.gl 9.x gives one device API over WebGPU and WebGL2 with no map chrome and no API keys. geoprims writes its own small layer set (the layer kinds in `hud-canvas`).

| Alternative | Why not |
|---|---|
| deck.gl | Good layer catalog, but larger. Its geospatial layers assume Web Mercator tiles; we need ellipsoidal densification and a globe. It could back `cell-set` layers later. |
| MapLibre GL JS 6 | WebGL2 only, map-first. Heavier than needed when the basemap is optional. |
| CesiumJS | Full 3D globe, but multiple MB. Overkill for vector overlays. |
| three.js | General-purpose. Projection and globe math would be ours anyway. |

- **Geometry comes from Wasm.** Densification, antimeridian splitting, and pole handling run in the core so the canvas draws exactly the math the tool computed. The renderer only projects to screen.
- **Canvas2D fallback** supports points, lines, polygons, and vector diagrams with no GPU.
- **Effects.** HUD post-processing (bloom from a blurred bright pass, persistence via previous-frame blend, scanlines) is a single optional pass, disabled by reduced motion or the settings toggle.

### W4. Basemap
- Default: bundled Natural Earth 110m (public domain) plus graticule, rendered as vector lines in the HUD style.
- Optional: self-hosted vector tiles (Protomaps PMTiles extract, OpenStreetMap-derived, ODbL attribution) served from `assets.geoprims.com` with range requests.
- Tile requests are capped at zoom ≤ 7, per the privacy spec's coarse-location rule (a z7 tile spans 2.8° of longitude). Higher zooms overzoom the z7 data. PMTiles range reads fetch whole-tile byte ranges only, so a request never identifies a sub-tile area.

### W5. Command palette search: uFuzzy with custom ranking
uFuzzy (~7.6 KB) indexes title, id, aliases, keywords, and group. The final score adds boosts for exact alias match, prefix match, pinned, and recency. At ~1,000 entries it searches well under 1 ms. Ranking quality is tested with a fixed query → expected-top-3 fixture set, which the command-palette scenarios seed.
- Paste-to-detect runs a detector chain in Wasm (H3, S2 token, geohash, quadkey, XYZ, Maidenhead, MGRS, coordinate notations, altimeter group) and returns all plausible interpretations.

### W6. Permalink encoding
Fragment = `#v1:` + base64url(deflate-raw(canonical JSON of inputs, units, view)). Compression uses the native `CompressionStream`, with a tiny fallback. The version prefix allows migrations; each tool's manifest can declare field renames for fragment migration.

### W7. PWA and caching: Workbox
- Precache: shell with an offline route renderer, catalog, search index, and Wasm (≤ 12 MB compressed). Docs pages are cached when visited or via an optional Docs pack.
- Runtime cache-first for immutable, content-hashed assets.
- Offline packs go to OPFS or the Cache API with integrity checks (data-assets spec).
- Updates use a "waiting" service worker and a user prompt.

### W8. Build-time math: Temml (LaTeX → MathML)
Formulas are authored in LaTeX in docs, rendered to MathML at build, and checked in the accessibility job. No runtime math library.

### W9. Audio: raw Web Audio API
A ~5 KB module synthesizes the fixed sound set with oscillators, noise buffers, and gain envelopes. It is lazy-loaded only when audio is enabled.

### W10. Testing stack
- End-to-end and cross-browser: Playwright (Chromium, Firefox, WebKit).
- Accessibility: axe-core, run in every theme mode.
- Performance: Lighthouse CI budgets on a sampled route set.
- Visual regression: canvas and HUD components, screenshot diffs per mode.
- Egress privacy: proxy capture (from the foundation change).

## Risks / Trade-offs

- **[luma.gl and deck.gl still call WebGPU experimental; API churn]** → Keep the renderer behind our own thin interface. Pin versions. WebGL2 is the tested baseline.
- **[800 pages × previews makes build time long]** → Incremental builds keyed on manifest hash. Preview images come from the headless Canvas2D renderer in parallel.
- **[HUD effects reduce contrast]** → Contrast is measured on rendered pixels, effects included (visual-theme spec). High-contrast mode disables effects.
- **[Permalinks leak inputs when users paste them into chat or email]** → The fragment keeps them off our servers, not away from recipients. The share dialog says so.
- **[Safari storage eviction breaks offline packs]** → Prompt to install. Show the eviction warning.

## Migration Plan

Not applicable (greenfield). The site launches with the tools that reach `stable` in the domain changes; experimental tools are visible but labeled.

## Open Questions

- Which self-hosted font pairing (monospace for numbers plus a humanist sans for prose): a design exploration inside task 2.1. Does not affect specs.
