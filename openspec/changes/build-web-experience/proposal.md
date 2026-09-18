## Why

Existing spatial and aviation calculators are scattered across ad-heavy sites, agency forms with server round trips, and paid mobile apps. None of them shows the math: you type numbers and get numbers back. geoprims gives humans one fast, keyboard-first, offline-capable place where every calculation is drawn on a live canvas, so a wrong input is visible immediately (a route crossing the wrong ocean, a wind vector pointing the wrong way).

Depends on: `establish-platform-foundation`.

## What Changes

- An **app shell**: statically pre-rendered page per tool, schema-driven input forms, instant recompute on input, shareable permalinks (inputs in the URL fragment), recent and pinned tools, tool chaining.
- A **command palette** (`/`) that fuzzy-searches the full catalog, aliases, and recent inputs in under 16 ms, plus global keyboard shortcuts.
- A **HUD canvas** as the hero element: 2D map, 3D globe, and vector-diagram modes that render each tool's declared visualization in real time, with an optional offline basemap.
- A **visual theme**: five modes (`hud` signature dark mode, `daylight`, `sunlight` for outdoor field use, `night` for cockpit use, `high-contrast`), all WCAG 2.2 AA, with HUD effects opt-in.
- **Audio feedback**: optional synthesized micro-clicks and tones, off by default, one-key mute.
- **Import and export**: GeoJSON, KML, GPX, CSV, WKT/WKB, and clipboard formats, plus batch mode over CSV.
- **Offline PWA**: installable, with per-domain offline packs for large datasets.
- **Documentation on every page**: formula, variables, references, accuracy, worked example, and edge cases, readable without JavaScript.

## Capabilities

### New Capabilities

- `web/app-shell`: Routing, tool pages, schema-driven forms, live recompute, permalinks, history, chaining, settings.
- `web/command-palette`: Fuzzy finder and global keyboard model.
- `web/hud-canvas`: Real-time 2D/3D/vector visualization of tool inputs and outputs.
- `web/visual-theme`: HUD design system, color modes, typography, motion, and accessibility.
- `web/audio-feedback`: Optional synthesized UI sounds.
- `web/io-formats`: File and clipboard import/export and batch processing.
- `web/offline-pwa`: Installation, caching, offline packs, and updates.
- `web/tool-docs`: Pre-rendered documentation, SEO metadata, and structured data per tool.

### Modified Capabilities

None.

## Non-goals

- No full GIS editor (layer management, styling, attribute tables). The canvas visualizes tool inputs and outputs; it is not QGIS.
- No proprietary basemaps or API-keyed services (Google, Mapbox, Esri).
- No native mobile apps in v1; the PWA covers mobile.
- No collaborative or cloud-saved sessions.

## Impact

- Adds the static site generator, the UI component library, the rendering engine, and the service worker.
- Adds end-to-end, visual regression, accessibility, and performance CI jobs.
- Adds hosting configuration (headers, caching, redirects) on a static host.
