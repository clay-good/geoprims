# Tasks

- [ ] 1 Add the sources to `data/sources-ledger.json` at their current editions (Barnes 2014 ×2, Lindsay 2016, O'Callaghan & Mark 1984, Tarboton 1997, Garbrecht & Martz 1997, Strahler 1957, Jenson & Domingue 1988, ESRI flow-direction and watershed docs as vendor, WhiteboxTools manual) → verify: the ledger gate passes and each `verificationNote` names the section read
- [ ] 2 Add WhiteboxTools (pinned release, MIT) and richdem (pinned, GPL, external process only) to `verify/Dockerfile` with smoke cases → verify: `verify/smoke.sh` runs a fill and a D8 pointer through each
- [ ] 3 `raster.hydrology.fill` (Priority-Flood ε, least-cost breach) → verify: the single-bowl, edge-pit, and nested-pit vectors; fill volume equals the oracle's to 1e-6 m³; NoData never changes
- [ ] 4 `raster.hydrology.flow-direction` (D8 with ESRI, WhiteboxTools, TauDEM encodings; D∞) → verify: ramp vectors; encoding tables pinned; 1,000 synthetic windows match WhiteboxTools exactly except documented ties
- [ ] 5 `raster.hydrology.flow-accumulation` (D8 and D∞) → verify: ramp rows 1…10; the conservation invariant on every test window; D8 and D∞ agree on a planar ramp
- [ ] 6 `raster.hydrology.streams` (threshold, Strahler order, GeoJSON links) → verify: the order-rule vector; link counts per order match WhiteboxTools on the GLO-30 crop
- [ ] 7 `raster.hydrology.watershed` (two snap rules, basin polygon, stats, `WATERSHED_TOUCHES_EDGE`) and `raster.hydrology.longest-flow-path` → verify: nested-basin invariant; the Jenson & Domingue grid's basins; the 270 m ramp path; the off-data `OUT_OF_DOMAIN` case
- [ ] 8 Window caps, memory declaration (512 MiB for the group), progress, and cancellation; calibrate time budgets through `dist/wasm` → verify: the four-tile phone scenario; cancellation within 100 ms on a 12.96 M-cell fill in Chromium
- [ ] 9 Register `WATERSHED_TOUCHES_EDGE` in `data/codes.json`; add the `hydrology` group to `data/taxonomy.json`; hubs, glossary, derivation notes, and the mobile and visual gates (the basin and streams draw on the map; the fill shows changed cells) → verify: the full gate sequence passes and each tool's questions rank it first
