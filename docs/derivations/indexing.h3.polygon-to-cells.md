<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# H3 polygon to cells (`indexing.h3.polygon-to-cells`)

## Method

H3 C's `polygonToCellsExperimental`, written on h3o's core API. The polygon (outer ring and holes, from points or GeoJSON) is tested by ray casting in latitude and longitude (radians), with H3 C's tie-breaking and antimeridian handling. The search starts at every H3 base cell. A conservative box either rejects a whole branch, proves all its descendants are inside, or sends the search to the next resolution. At the requested resolution, one of H3's four containment rules keeps the cell. Before filling, a bounding-box estimate refuses fills that would be too large.

## Equations

- Center mode: keep cell c when its center lies inside the outer ring and outside every hole.
- Full mode: keep c when all its boundary vertices lie inside and no polygon edge crosses its boundary.
- Overlapping mode: keep c when a boundary vertex is inside, a polygon vertex is inside c, or their edges cross.
- Overlapping-bbox mode: apply the overlap tests, then also keep c when H3's buffered descendant box contains, is contained by, or crosses the polygon.
- Traversal: descend only when the polygon overlaps a conservative descendant box; when the polygon wholly contains that box, emit every descendant at the requested resolution without testing them separately.
- Estimate: bounding-box area / average cell area at the resolution.

## Symbols and units

Vertices in degrees (WGS 84), resolution 0–15, cells as 15-character H3 indexes. Area is in km².

## Domain

Polygons with holes, including ones that cross the antimeridian, at resolutions 0 to 15. Fills with an estimate above 5,000,000 cells or more than 1,000,000 cells are refused with LIMIT_EXCEEDED.

## Approximations

Edges are straight in latitude and longitude, as in H3 C, not great circles. For long edges this differs from a geodesic polygon.

## Worked example

- sourcePublisher: Uber Technologies (H3 project)
- sourceTitle: H3 C test suite, testPolygonToCells.c
- sourceEdition: H3 v4.4.1
- sourceLocator: polygonToCells on sfGeoPolygon at resolution 9 gives 1,253 cells; holeGeoPolygon gives 1,214
- independent: yes
- inputs: the six-vertex San Francisco polygon (radians converted to degrees), resolution 9, center containment; and the same with the triangular hole
- outputs: 1,253 cells; 1,214 cells
- tolerance: exact counts
- verifiedBy: golden vectors v022 and v023, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/h3_fill_parity.rs`: 60 random polygons with holes and antimeridian crossings, 15 polar rings, and 44 resolution 0 and 1 cases, identical cell sets to H3 C 4.4.1 in all four modes
- `tools/vectors/gen_h3_fill.py`: regenerates that fixture; the full 1,000-polygon run also matches exactly in all four modes
- `core/vectors/indexing.h3.polygon-to-cells.jsonl`: 23 vectors from H3 C via h3-py 4.4.2

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `chooser_and_fill_invariants`: fills nest (full inside center inside overlapping), and the count matches the cells returned
- Every polar row is asserted in all four modes; no mode is exempted from the parity gate.
