<!-- Derivation note (trust/correctness-program layer A). -->
# Crosshatch survey grid (`drone.mission.crosshatch`)

## Method

A crosshatch, or double grid, flies an area twice: once on parallel lines, then again on lines at right angles to them. The second pass sees the faces the first one looked along, which a 3D reconstruction needs. Each pass is the serpentine of `drone.mission.survey-grid`, built on a local plane, and the operation is that tool's own code with its crosshatch switch held on.

## Equations

For each pass, on axes turned to the pass direction:

- Lines n = ⌈extent across the lines ÷ spacing⌉ + 1, centered on the extent so that (n − 1) × spacing ≥ extent
- Photo spacings on a line of length l: ⌈l ÷ photo spacing⌉
- Second pass direction = first pass direction + 90°
- Totals (lines, photos, survey length, path length) = the sums over both passes, plus the transit between them in the path length

## Symbols and units

Line and photo spacing in meters or feet; the direction as an azimuth in degrees; the area as corners in latitude and longitude.

## Domain

As the survey grid: at least three corners, a line spacing of 0.5 m or more, a photo spacing of 0.1 m or more, and no more than 5,000 lines per pass.

## Approximations

The pattern is laid out on a local plane tangent at the area, then placed back on the ellipsoid, which is the survey grid's stated method.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-drone-suite specification
- sourceEdition: 2026
- sourceLocator: drone/mission-patterns, requirement "Survey grid" (optional crosshatch); the line counts are checked by hand, not taken from a published table
- independent: no
- inputs: a 199.7 m by 270.2 m rectangle at 45 m line spacing
- outputs: 6 lines across the 199.7 m side (⌈4.44⌉ + 1) and 8 across the 270.2 m side (⌈6.00⌉ + 1), 14 in all
- tolerance: exact
- verifiedBy: golden vector v001 and the `focused_mission_and_ops_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `crosshatch_vectors`: 20 rectangles from the equator to 64° latitude, their sides computed from the ellipsoid's radii of curvature in Python and the line count from the rule above.
- `core/vectors/drone.mission.crosshatch.jsonl`: those vectors and two refusal cases.
- The integration suite requires the whole result to equal `drone.mission.survey-grid` with crosshatch on, and its lines, photos, and survey length to equal the sum of two single passes 90° apart.

## Invariants

- Lines, photos, and survey length are the sums of the two passes.
- Turning the first pass by 90° gives the same totals.
- A square area has the same number of lines in each pass.
