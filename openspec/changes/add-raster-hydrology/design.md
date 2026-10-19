## Context

The raster module already decodes GeoTIFFs in Rust through a private file handle (`add-spatial-indexing-and-raster` IR6, IR7), reads GLO-30 tiles with a one-cell margin, and verifies derivatives against `gdaldem`. Hydrology needs whole-window algorithms (a priority queue over the DEM) rather than 3 × 3 kernels, so memory, not arithmetic, sets the limits. Research for this change: the sources in `proposal.md`, gathered 2026-10-19.

## Goals / Non-Goals

**Goals:**
- Standard, named algorithms whose output matches WhiteboxTools and richdem cell for cell.
- Honest edge behavior: edges are outlets, NoData is outside, and every result says so.

**Non-Goals:**
- Design hydrology. Multi-flow-direction routing beyond D∞. Stream burning.

## Decisions

### H1. Priority-Flood with ε-slope as the default conditioning
Depressions are filled with Priority-Flood (Barnes, Lehman & Mulla 2014, Algorithm 3), seeding the queue with every edge cell and every cell adjacent to NoData, raising each visited neighbor to at least the popped elevation plus ε (default 1e-5 of the vertical unit, so filled areas drain). Breaching (Lindsay 2016, least-cost with a maximum breach depth and length) is an option for users who need relief preserved. Both produce a conditioned DEM that D8 can route without flats.
- *Alternative: Planchon & Darboux 2001.* Correct but slower (iterative), and Priority-Flood is the algorithm the oracles implement.
- *Alternative: Garbrecht & Martz flat resolution after a plain fill.* Equivalent routing on flats; ε-fill is simpler to state and verify. The result names the method either way.

### H2. D8 is the default; D-infinity is an option
D8 (steepest descent to one of eight neighbors, diagonal distance √2 × cell size) is what every oracle agrees on and what watershed delineation needs (a single pointer graph). D∞ (Tarboton 1997) is offered for accumulation only, where its split flow gives smoother upslope areas on hillslopes. Watershed and longest path always use D8.

Direction encoding: results carry directions as the ESRI powers-of-two table (E = 1, SE = 2, S = 4, SW = 8, W = 16, NW = 32, N = 64, NE = 128; 0 = no outflow), because it is the most widely documented and what pysheds and ArcGIS users expect. The tool offers the WhiteboxTools and TauDEM encodings as output options, named, and the manifest documents all three.

### H3. Edges, NoData, and the lower-bound caveat
- A cell on the raster edge, or next to NoData, drains off the grid when it has no lower neighbor (ESRI and GRASS behavior). Such cells are outlets.
- NoData cells never receive or give flow and are never filled.
- Accumulation near the edge counts only cells inside the window, so the result sentence says "near the edge of the data this is a lower bound," and the watershed tool warns `WATERSHED_TOUCHES_EDGE` (new, `accuracy`) when the delineated basin reaches the window edge, because the true basin may continue beyond it.

### H4. Watershed from a pour point
The pour point is a lat/lon (or a row/column in the DEM). Two snapping rules, named in the result: "nearest stream cell" (Jenson & Domingue 1988, the default, with the stream threshold given) or "highest accumulation within a distance" (ESRI Snap Pour Point). The basin is the set of cells whose D8 path reaches the snapped cell. The result returns the basin polygon (cut at the window edge), area (geodesic, from `geometry/computational`), outlet, relief, mean slope, the stream order at the outlet, and the longest flow path.

### H5. Memory and size limits
Priority-Flood needs the DEM as float32, a visited bitmap, a pointer byte per cell, an accumulation grid (u32 cell counts; area is count × cell area), and a queue of up to n entries. Peak memory is about 14 bytes per cell plus the queue. Caps, checked before allocation and returned as `LIMIT_EXCEEDED`:

| Host | Window cap | Reason |
|---|---|---|
| Browser, desktop profile | 25,000,000 cells (5,000 × 5,000) | ≈ 400 MB peak inside the 64 MiB-per-invocation default raised to a declared 512 MiB for this group |
| Browser, mobile gate | 10,000,000 cells | WebKit memory pressure on phones |
| MCP | 25,000,000 cells | same as desktop |

A GLO-30 window of 1° × 1° is 3,600 × 3,600 = 12.96 M cells, so one tile fits every cap; a 2 × 2 tile window exceeds the mobile cap and the tool says so with the size. Time budgets are measured through `dist/wasm` on the reference profile and declared per tool as data-proportional.

### H6. Verification
- **Differential:** WhiteboxTools (MIT, Rust; `FillDepressions`, `BreachDepressionsLeastCost`, `D8Pointer`, `DInfPointer`, `D8FlowAccumulation`, `StrahlerStreamOrder`, `Watershed`, `SnapPourPoints`, `JensonSnapPourPoints`, `LongestFlowpath`) is the primary oracle, run in `verify/`. richdem is the second oracle, run only as an external GPL process; none of its code or test grids is copied.
- **Golden vectors:** the Jenson & Domingue 1988 10 × 10 example grid as reproduced in ESRI's flow-direction and watershed documentation (fill, directions, accumulation, basins), transcribed and confirmed by both oracles; a 10 × 10 all-east ramp (accumulation rows 10 … 1); synthetic bowls (one pit, two nested pits, a plateau with one outlet) whose fill result is the spill elevation; and one 512 × 512 GLO-30 crop over a small basin with WhiteboxTools outputs.
- **Invariants:** after fill, every cell has a non-ascending path to an edge; accumulation at any cell equals 1 plus the sum over cells pointing to it; the watershed of a cell contains the watersheds of every cell upstream of it; D8 and D∞ accumulation agree exactly on a planar ramp.

## Tool inventory

| Group | Operations | Endpoints | Reference | Independent vector |
|---|---|---|---|---|
| `raster.hydrology` | fill, flow-direction, flow-accumulation, streams, watershed, longest-flow-path | 6 | Barnes 2014; Lindsay 2016; O'Callaghan & Mark 1984; Tarboton 1997; Strahler 1957; Jenson & Domingue 1988 | ESRI's reproduction of the Jenson & Domingue grid; WhiteboxTools on the GLO-30 crop |

Total: 6 operations, 6 endpoints. The raster rollup moves from 30 to 36 operations.

## Risks / Trade-offs

- **[Users read a GLO-30 watershed as bare-earth hydrology]** → Every result repeats that GLO-30 is a surface model and suggests a bare-earth DEM (USGS 3DEP) through the file bridge.
- **[Oracles disagree on flats or ties]** → Pin the ε-fill and the tie rule (lowest neighbor index in ESRI order) and show a `tie` count; vectors use oracle-agreed cells only, with the disagreement documented in the derivation note.
- **[A phone runs out of memory]** → Pre-check caps, cancel within 100 ms, and tell the reader the window size that fits.

## Migration Plan

Not applicable. New tools start experimental and follow the promotion recipe.

## Open Questions

- Whether the ESRI documentation still reproduces the Jenson & Domingue grid at implementation time; if not, the vector cites the paper and both oracles.
