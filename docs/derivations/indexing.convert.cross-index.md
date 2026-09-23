<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# One point in every index (`indexing.convert.cross-index`)

## Method

A point has no single "address". It has seven, one per indexing system, and each system offers a ladder of resolutions rather than a single cell. So the question this tool answers is not "what is the cell" but "which rung of each ladder is closest to the size I care about, and how big is it really there".

Two decisions do the work.

**How a cell's size is measured.** A cell is a hexagon, a square, or a lat/lon rectangle, so the systems are not directly comparable until they are reduced to one number. That number is the side of a square of the same area — for a rectangle w by h, √(wh). For the systems whose cells are defined in degrees (geohash, Plus Code, Maidenhead, MGRS) or in Web Mercator (map tiles), the size depends on where you are, so it is computed at the given latitude rather than taken from a table: a geohash cell at 60° north is half as wide in metres as the same cell at the equator.

**Which resolution is closest.** Sizes step by factors — a geohash character is a factor of 8 or 4, an H3 resolution a factor of 7, a tile zoom a factor of 2 — so closeness is measured by ratio, not difference. The resolution chosen is the one minimising |log(size / target)|, which treats being twice too big and half too small as equally far off. Picking by plain difference would bias every answer toward the coarser rung.

## Equations

- Cell size = √(width · height) in metres, at the given latitude.
- Metres per degree at latitude φ: meridian M(φ) = a(1−e²)/(1−e² sin²φ)^{3/2} · π/180; parallel N(φ) cos φ · π/180, with N = a/√(1−e² sin²φ).
- Geohash at precision p: 360° / 2^⌈5p/2⌉ wide, 180° / 2^⌊5p/2⌋ tall.
- Plus Code: 20° at 2 characters, ÷20 per pair to 10 characters, then ÷5 in latitude and ÷4 in longitude per extra character.
- Maidenhead: 10° by 20° at 2 characters, alternating ÷24 and ÷10 per pair.
- Map tile at zoom z: 40,075,016.6856 · cos φ / 2^z metres.
- MGRS at d digits: 100,000 / 10^d metres.
- S2 at level L: √(4π/(6·4^L)) · R, with R the Earth's mean radius.
- H3 at resolution r: √(average hexagon area at r).
- Chosen resolution = argmin |ln(size / target)|.

## Symbols and units

`lat` and `lon` place the point; `target_size` is the cell size you want. Out comes `cells`, one row per system with its cell, its resolution and the size that resolution actually has at this latitude, and `closest_match`, the system that came nearest.

## Domain

Any point. Any target size from about a metre to a thousand kilometres; beyond either end every system pins to its finest or coarsest rung, which is the honest answer.

## Approximations

The cell size is a single representative number for a shape that is not a square, so it is exact for no cell and fair for all of them. H3 hexagons vary in area within a resolution — the twelve pentagons most of all — so the H3 figure is the published average, not this particular cell. Map tile and geohash sizes are exact at the given latitude and change as you move north or south within the cell.

## Worked example

- sourcePublisher: Uber (H3); Google (Plus Codes, S2); OpenStreetMap contributors (tiles); NGA (MGRS); IARU (Maidenhead)
- sourceTitle: the published resolution tables and cell definitions of each system
- sourceEdition: H3 average-area table; the Plus Code and Maidenhead character steps; the Web Mercator circumference; MGRS digit precision
- sourceLocator: each system's own definition of how a resolution step changes cell size
- independent: yes
- inputs: 21 points, from the equator to 80° north and south, with target sizes from 1 m to 1,000 km
- outputs: the chosen resolution in all seven systems, and the cell size for geohash, tiles and S2
- tolerance: 1e-9 relative on the sizes, exact on the resolutions
- verifiedBy: golden vectors v001 to v026, run by the core on every build
- verifiedOn: 2026-09-23

The reference in `tools/vectors/gen_cross.py` and `gen_cross_more.py` restates each system's size ladder from its own definition in Python and applies the same log-distance rule, rather than reading any of it back from the core. The latitudes matter here: at 80° a map tile is a sixth of its equatorial width while an S2 cell is unchanged, so a target that picks matching rungs at the equator picks rungs two steps apart up there. Those cases are in the vectors.

## Differential tests

- `tools/vectors/gen_cross.py` and `gen_cross_more.py`: all 26 vectors, from each system's published definition
- `core/crates/gp-indexing/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/indexing.convert.cross-index.jsonl`: 26 vectors across latitude and six orders of magnitude of target size

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `cross_index_invariants`: every system reports a cell and a resolution at every point tried, so no row is silently dropped; `closest_match` really is the system whose size is nearest the target by ratio, recomputed from the reported sizes rather than trusted; asking for a smaller target never returns a coarser resolution in any system, which is the monotonicity the ladder guarantees; for the systems measured in degrees or Web Mercator, reaching the same target at 60° north takes a coarser rung than at the equator, and a tile zoom steps exactly once, since a degree of longitude is half as long there; S2 and H3 are defined on the sphere and do not move at all. It is the rung that carries the effect, not the size — the size stays near the target at both latitudes, which is the point of the tool; and each reported cell decodes back to the point it came from, through that system's own decoder, so the cell and the size describe the same place
