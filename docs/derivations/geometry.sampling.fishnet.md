<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Fishnet grid (`geometry.sampling.fishnet`)

## Method

A rectangle grid laid out the way QGIS Create grid lays one out, in the UTM zone plane of the polygon (as in `geometry.sampling.random-points`, step 1).

1. **Origin and frame.** The origin is the top-left corner (least x, greatest y) of the outline's bounding box on the plane. A rotation θ turns the grid clockwise about it, and the polygon is turned back by θ to find its extent [x₀, x₁] × [y₀, y₁] in the grid's frame.
2. **Cells.** There are ⌈(x₁ − x₀)/w − 10⁻⁹⌉ columns and ⌈(y₁ − y₀)/h − 10⁻⁹⌉ rows. The 10⁻⁹ keeps an extent a whole number of cells wide from gaining a cell through rounding. Cell (row r, column c) spans x₀ + c·w to x₀ + (c + 1)·w and y₁ − r·h down to y₁ − (r + 1)·h. Cells run column by column from the left, top to bottom, numbered from 0, as in QGIS.
3. **Inside fraction.** The polygon (holes included) is cut into triangles by the ear clipping of the random-points note, in the grid's frame. Each cell's inside fraction is the area of those triangles clipped to the cell (Sutherland-Hodgman) over the cell's area. Clip mode keeps only cells with a fraction above zero.
4. **Scale.** The plane's point scale is taken at every polygon corner and the extent's corners, and reported as a range in parts per million from true.

## Equations

- Cell area: w·h, in plane m².
- Inside fraction: Σ area(triangle ∩ cell) / (w·h).
- Scale: k(φ, λ) of the transverse Mercator, (k − 1)·10⁶ ppm.

## Symbols and units

- w, h: cell width and height in UTM plane meters, as QGIS takes them in the CRS's units.
- θ: rotation in degrees, clockwise.
- Centers and outlines: in degrees.

## Domain

- At most 50,000 cells over the extent, or `LIMIT_EXCEEDED`.
- The polygon spans at most 500 km and has at most 2,000 corners.

## Approximations

Cells are exact rectangles on the UTM plane. On the ground their sides differ by the plane's scale, which the result reports. Clipped cells keep their whole outline. The inside fraction says how much of each lies in the polygon.

## Worked example

- sourcePublisher: geoprims (the change's own scenario)
- sourceTitle: A 100 m fishnet over a 1 km × 1 km square in its UTM zone
- sourceEdition: 2026-10-20
- sourceLocator: add-sampling-and-grids spec, "QGIS parity": 100 cells whose corners fall on the 100 m lines
- independent: no (QGIS itself is not run here)
- inputs: a square of 1,000 m on the zone 13 plane, width 100 m
- outputs: 100 cells, every corner within 1 mm of a 100 m line, the first cell's center 50 m right of and 50 m below the top-left corner
- tolerance: 1 mm
- verifiedBy: `fishnet_and_hex_scenarios`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_grid_designs.py fishnet`: the layout reimplemented in Python with PROJ's transverse Mercator. Inside fractions come independently from Shapely (GEOS) intersection areas, not triangle clipping. It covers 20 cases over five polygons with square and oblong cells, rotations, extent and clip modes.
  - Counts, rows, and columns agree exactly.
  - Centers and the first cell's corners agree within 1e-9°.
  - Inside fractions agree within 1e-7.
- `core/vectors/geometry.sampling.fishnet.jsonl`: those vectors and the too-many-cells refusal, run through the core on every build.

## Invariants

- `core/crates/gp-geometry/tests/sampling.rs` `fishnet_and_hex_scenarios`:
  - inside fractions times the cell area add to the polygon's area (a 1 km square with a hole) within 0.1%, unturned and turned 17°;
  - clip mode keeps exactly the cells with any part inside.
