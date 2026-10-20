<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Hexagon grid (`geometry.sampling.hex-grid`)

## Method

QGIS Create grid's hexagon layout on the polygon's UTM plane, with the origin, rotation, inside fractions, and scale range of `geometry.sampling.fishnet`.

1. **Hexagons.** Regular flat-top hexagons of height s (flat side to flat side). With lo = 0.288675134594813·s and hi = 0.577350269189626·s, a hexagon is 2·hi = 1.1547·s wide, and columns are lo + hi = 0.866·s apart.
2. **Lattice.** There are ⌈width/(0.866 s) − 10⁻⁹⌉ columns and ⌈height/s − 10⁻⁹⌉ rows. Column c's left corner is at x₀ + c·0.866 s. Row r's top is at y₁ − r·s, and odd columns sit s/2 lower. Cells run column by column, top to bottom, numbered from 0, as in QGIS.
3. **Edges.** Started with a left corner on the extent's edge, QGIS's hexagons leave the extent's ragged edges uncovered. So the lattice cells one row above, one row below, one column before, and one column after are kept where they reach into the extent. Their rows and columns read −1 or one past the last. QGIS's own cells are a subset, with the same coordinates.
4. **Inside fraction.** The polygon's triangles are clipped to each hexagon by Sutherland-Hodgman against its six sides.

## Equations

- Hexagon corners, counterclockwise from the left: (x_a, y_b), (x_b, y_c), (x_c, y_c), (x_d, y_b), (x_c, y_a), (x_b, y_a).
  - x_a = x₀ + c·(lo + hi), x_b = x_a + hi − lo, x_c = x_a + lo + hi, x_d = x_c + hi − lo.
  - y_a = y₁ − (2r + k)·s/2, y_b = y_a − s/2, y_c = y_a − s, with k = c mod 2.
- Cell area: (√3/2)·s². Each side is s/√3.

## Symbols and units

- s: hexagon height in UTM plane meters.
- θ: rotation in degrees, clockwise.
- Centers and outlines: in degrees.

## Domain

- At most 50,000 cells over the extent, or `LIMIT_EXCEEDED`.
- The polygon spans at most 500 km and has at most 2,000 corners.

## Approximations

Hexagons are exact on the UTM plane. On the ground they differ by the plane's scale, reported as a range.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: 50 m hexagons over a 1 km square
- sourceEdition: 2026-10-20
- sourceLocator: Every side measured on the zone 13 plane through gp-geo's own UTM forward
- independent: no (QGIS itself is not run here)
- inputs: a square of 1,000 m on the zone 13 plane, s = 50 m
- outputs: six sides of 50/√3 = 28.868 m on every hexagon checked
- tolerance: 1 mm
- verifiedBy: `fishnet_and_hex_scenarios`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_grid_designs.py hex`: the layout and edge rule reimplemented in Python with PROJ's transverse Mercator, and inside fractions from Shapely (GEOS) intersection areas. It covers 20 cases over five polygons with rotations, extent and clip modes. Counts, rows, and columns agree exactly, centers and corners within 1e-9°, and inside fractions within 1e-7.
- `core/vectors/geometry.sampling.hex-grid.jsonl`: those vectors and the too-many-cells refusal, run through the core on every build.

## Invariants

- `core/crates/gp-geometry/tests/sampling.rs` `fishnet_and_hex_scenarios`:
  - inside fractions times the cell area add to the polygon's area within 0.1%, unturned and turned 17°, which holds only because the edge cells cover the extent;
  - clip mode keeps exactly the reaching cells;
  - every hexagon is regular.
