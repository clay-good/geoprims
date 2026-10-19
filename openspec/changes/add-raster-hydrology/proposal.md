## Why

The terrain tools answer "how steep, which way, what can I see," but not "where does the water go." That question comes up on every site: a drone operator mapping a parcel is asked where runoff leaves it, a surveyor staking a culvert needs the catchment above it, a developer wants a stream network from a DEM without installing GRASS. Today the answer means a desktop GIS, or an online service that uploads the DEM. The raster suite deferred hydrology to "after v1" (`add-spatial-indexing-and-raster`, Non-goals); this change adds it, on the device, from the same DEM sources and the same raster bridge.

Depends on: `add-spatial-indexing-and-raster` (raster module, DEM sources, private file bridge), `add-navigation-and-geometry` (polygons, areas).

## What Changes

Adds the `raster.hydrology` group: six operations, each a standard step of DEM hydrology with a named algorithm and a published reference, verified against WhiteboxTools and richdem.

| Tool | Question it answers | Method |
|---|---|---|
| `raster.hydrology.fill` | Which cells are pits, and what does the DEM look like with them removed? | Priority-Flood fill with ε-slope (Barnes, Lehman & Mulla 2014), or least-cost breaching (Lindsay 2016) |
| `raster.hydrology.flow-direction` | Which way does each cell drain? | D8 (O'Callaghan & Mark 1984) by default; D-infinity (Tarboton 1997) as an option |
| `raster.hydrology.flow-accumulation` | How much area drains through each cell? | Upslope cell count and area over the D8 or D∞ graph |
| `raster.hydrology.streams` | Where are the channels, and what order is each? | Accumulation threshold, Strahler order (Strahler 1957), stream links as lines |
| `raster.hydrology.watershed` | What area drains to this point? | Pour point snapped to the stream or to the highest accumulation within a distance, upstream traversal; area, outlet, relief, mean slope |
| `raster.hydrology.longest-flow-path` | How long is the longest flow path to the outlet? | Longest upslope D8 path from the outlet |

Every result states the source DEM, its resolution, that GLO-30 is a surface model, the fill or breach method, the direction encoding, and that raster edges are treated as outlets so accumulation near edges is a lower bound.

## Capabilities

### New Capabilities

- `raster/hydrology`: Depression removal, flow direction and accumulation, stream extraction and order, watershed delineation, and longest flow path on local DEMs.

### Modified Capabilities

None.

## Non-goals

- Rainfall-runoff, peak flow, time of concentration, or any design hydrology (rational method, SCS curve number). Those are engineering judgments with jurisdictional tables.
- Stream burning from a vector network, or hydrologic conditioning beyond fill and breach.
- Global or basin-scale runs. Browser caps apply (design H5).

## Impact

- `core/gp-raster` grows by the hydrology operations; its 400 KB budget is checked.
- `data/taxonomy.json` adds the `hydrology` group under `raster`.
- `verify/` gains WhiteboxTools (MIT, Rust) and richdem (GPL-3, run as an external process only) as differential oracles.
- New sources-ledger rows: Barnes, Lehman & Mulla 2014 (both papers), Lindsay 2016, O'Callaghan & Mark 1984, Tarboton 1997, Garbrecht & Martz 1997, Strahler 1957, Jenson & Domingue 1988, the ESRI flow-direction and watershed documentation (vendor, for the encoding and the worked grid), the WhiteboxTools manual.
