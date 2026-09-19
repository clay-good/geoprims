# apps/web

The geoprims static website: Astro 7 with Svelte 5 islands, built entirely ahead of time. Every page carries its real worked-example answer in the HTML, computed at build time by the same Wasm modules the browser loads.

**Making or changing a page? Start with the page template:** [`openspec/changes/build-web-experience/specs/web/page-template/spec.md`](../../openspec/changes/build-web-experience/specs/web/page-template/spec.md). It fixes the page anatomy, the self-service patterns, the component classes, and the wording rules, so every page feels like the same tool.

```bash
npm run build
```

```bash
npm install --prefix apps/web
```

```bash
npm run build --prefix apps/web
```

```bash
npm test --prefix apps/web
```

The first command runs at the repository root and builds the Wasm modules and catalog into `dist/`. The web build copies them into `public/`.

| Path | What |
|---|---|
| `src/pages/` | Home, domain hubs, group hubs, and one page per tool id (route map in `contracts/routes-and-urls`) |
| `src/components/ToolApp.svelte` | The tool island: schema-driven form (list inputs edit as one row per line, comma- or tab-separated), live answer card (core-rendered `display` and `summary`), warnings ordered by severity, copy actions, permalinks (`#v1:` via the core `link` module), Clear and Try the example |
| `src/components/PageHeader.astro`, `ListFilter.astro`, `ToolCards.astro` | The page-template building blocks: breadcrumbs, `h1`, and purpose line; the list filter (`?q=`, count, no-match search); and the tool card grid |
| `src/lib/compute.worker.js` | Runs the Wasm modules off the main thread; stale results are dropped |
| `src/styles/global.css` | Atlas design tokens: paper (the default) and ink, and the Geist fonts |
| `test/build.test.mjs` | One page per endpoint, answer in the HTML, canonical and noindex rules, no third-party requests |

Generated endpoints (like `/units/speed/kt-to-mph/`) canonicalize to their parent operation. Experimental tools are `noindex` until they have their full content.

Not built yet: the map canvas and animated scenes, offline packs and the pack manager, import and export, docs pages, and Playwright end-to-end suites. `node scripts/serve.mjs` serves the build with its production headers and CSP.
