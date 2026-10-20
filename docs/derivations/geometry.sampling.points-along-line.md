<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Points along a line (`geometry.sampling.points-along-line`)

## Method

Points at a fixed interval of plane length along a polyline, from a start offset, as QGIS's "Points along geometry" places them.

1. **Plane.** The line is carried to the UTM zone plane of its mean corner, as for the polygon tools, with its order kept.
2. **Cumulative length.** The plane length runs along the segments.
3. **Points.** Points fall at t = start + i·interval for i = 0, 1, … while t ≤ the line's plane length, so the last is kept only if it lies on the line. Each point sits on its segment at the fraction (t − cum_seg) / segment length.
4. **Lengths.** The geodesic length is the sum of Karney's inverse over the segments, shown beside the plane length so the scale difference is visible.

## Equations

- Count = ⌊(L − start) / interval⌋ + 1 when start ≤ L, else 0, where L is the plane length.
- Point i: on the segment whose cumulative length brackets tᵢ = start + i·interval.

## Symbols and units

- Line: corners in degrees, at least 2.
- Interval and start: in m, measured on the plane.
- Lengths: in m. Points: in degrees, with their plane distance from the start.

## Domain

- At most 100,000 points, or `LIMIT_EXCEEDED`.
- The line spans at most 500 km.
- The start offset cannot be negative.

## Approximations

Distances are on the UTM plane, as in GIS tools, so they run short or long of the ground by the zone's scale: from 0.9996 on the central meridian to about 1.001 at a zone's edge. The ground length is reported beside the plane length.

## Worked example

- sourcePublisher: geoprims (the change's own scenario)
- sourceTitle: Points every 30 m along a 100 m line
- sourceEdition: 2026-10-20
- sourceLocator: add-sampling-and-grids spec, "Last point rule": points at 0, 30, 60, and 90 m, none at 120 m
- independent: no (QGIS output, the spec's reference, is not reproduced here)
- inputs: a line 0.0009° north from 40° N, 105° W; interval 30 m
- outputs: points at 0, 30, 60, and 90 m
- tolerance: exact
- verifiedBy: `points_along_line_last_point_rule`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_grid_designs.py along`: the method reimplemented in Python with PROJ's transverse Mercator and pyproj.Geod. It covers 20 lines of 2 to 6 corners with start offsets. Counts agree exactly, points within 1e-9°, distances within 1e-9 m, and lengths within 1e-6 m.
- `core/vectors/geometry.sampling.points-along-line.jsonl`: those vectors and the too-many-points refusal, run through the core on every build.

## Invariants

- `core/crates/gp-geometry/tests/sampling.rs` `points_along_line_last_point_rule`:
  - points fall at 0, 30, 60, and 90 m on a 100 m line;
  - the plane length is just under the ground length near the central meridian;
  - a start offset shifts every point, and the count follows the formula above.
