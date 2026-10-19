## Context

Research for the wave is in each change's proposal and design (sources gathered 2026-10-19 from the issuers, the FAA and eCFR, NGA, NOAA, USNO, USACE, the OSGeo PROJ-data repository, and the hydrology and sampling literature). The product today: 357 tool ids across ten domains, 274 stable, eleven workflows, 33 explainers, live at geoprims.com since September 23, 2026.

## Goals / Non-Goals

**Goals:**
- Ship the wave in an order that returns value early and keeps every gate green.

**Non-Goals:**
- Any change to the two-surface, no-server, no-live-data rules.

## Decisions

### W1. Order

| # | Change | Why here | Adds | Cost drivers |
|---|---|---|---|---|
| 1 | `add-equipment-profiles` | Web-only, no core change; every later drone and survey tool benefits | 0 operations | Profile data file, one panel, MCP argument |
| 2 | `add-aviation-practical-additions` | Daily pilot questions; small exact tools; regulations are data | +10 aviation | Reading the TPP legend for the circling table |
| 3 | `add-photogrammetry-classics` | Four short formulas with textbook vectors; teaches the overlap presets | +4 drone | None |
| 4 | `add-survey-partition-and-adjustment` | Three office jobs; reuses the Cholesky solver and plane COGO | +3 survey | Textbook vectors to confirm at the source |
| 5 | `add-sampling-and-grids` | Serves drone, survey, and developers; first seeded PRNG in the core | +7 geometry | GPL oracles in `verify/` only |
| 6 | `add-marine-navigation` | A new audience on existing math; tide engine is the one real new engine | +15 navigation | Schureman implementation; NOAA parity; toolset split |
| 7 | `add-celestial-and-moon` | New `gp-astro` crate shared by three domains; USNO oracle | +5 navigation, +6 time, +2 survey | Series size vs module budgets |
| 8 | `add-raster-hydrology` | Whole-window algorithms; memory caps; new oracles in the container | +6 raster | WhiteboxTools and richdem in CI; Wasm memory on phones |
| 9 | `add-international-grids` | Largest data footprint; licenses recorded per asset | +26 geodesy | Asset pipelines, tiling, offline packs, `/licenses` |

Total: +84 operations, moving the target rollup from 514 to 598. Public counts still come from the build and count stable tools only.

### W2. Entry rule
A change starts when the domain's Phase 2 promotions are not blocked by it and the change's first tool can reach stable within the recipe (derivation note, independent worked example, dimension lint, bounds fuzz, invariants, differential, example parity, both surfaces). A change whose oracle is not yet in the differential container (hydrology, international grids) starts with the container task.

### W3. Budgets to watch

| Budget | Change | Mitigation named in the change |
|---|---|---|
| `gp-navigation` 400 KB | marine, celestial | tide engine or `gp-astro` split into its own module behind the same ids |
| `gp-raster` memory | hydrology | 512 MiB declared for the group; 25 M and 10 M cell caps |
| MCP toolset caps (40) | marine, international grids | `navigation` → `navigation` + `marine`; `projections` checked |
| Asset egress | international grids | 1° tiles, offline packs by country, 50 MB confirmation |
| Shell bundle 90 KB | equipment profiles | the panel loads with the tool, not the shell |

### W4. Alternatives considered
- *Ship the nine as one change.* Rejected: one failing gate would hold everything.
- *Demand-rank by Search Console first.* The console is not yet verified (value log, September 2026); the order above uses cost and the practitioner review's tiers instead, and the 90-day review may reorder 5 to 9.

## Risks / Trade-offs

- **[The wave crowds out Phase 2 promotions]** → W2's entry rule; promotions keep priority in each domain.
- **[Nine changes drift from each other's assumptions]** → Shared items are named once: profiles (change 1) feed changes 3, 6, and 7; Bowditch sources (change 6) feed change 7.

## Migration Plan

Not applicable.

## Open Questions

None.
