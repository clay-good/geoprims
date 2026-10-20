<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Area partition (`survey.cogo.area-partition`)

## Method

The cut area is a function of one unknown, and Brent's method finds the value that gives the required area.

- **Parallel to a side:** the unknown is the distance d from the side to the line. The area on the side's side of the line comes from clipping the figure by a half-plane.
- **At a bearing:** the same, with the line's direction set by the bearing. The sweep starts at the end of the figure nearer the chosen side.
- **Through a fixed point:** the fixed point sits on a side at a given distance from its first corner. The unknown is how far the free end has moved along the boundary, going forward in corner order. The cut piece is the fixed point, the corners passed, and the free end, and its area comes from the coordinate method.

Once the line is found, the figure is split at its two ends, and both pieces are measured by the coordinate method. When a parallel line ends on the two neighbor sides, the cut is a trapezoid, and the quadratic is shown so the answer can be checked by hand.

## Equations

- Coordinate method: 2A = Σ (xᵢ − x₀)(yᵢ₊₁ − y₀) − (xᵢ₊₁ − x₀)(yᵢ − y₀).
- Half-plane clip: keep the part of the figure where s(p) = n̂ · (p − V_k) ≤ t, where n̂ is the unit normal to the line.
- Trapezoid case: A(d) = L·d − d²(cot α + cot β)/2, so (cot α + cot β)/2 · d² − L·d + A = 0. Here L is the side length and α, β are the interior angles at its ends.
- Brent's method on [min s, max s], or on [0, perimeter] for the through-point case. The search runs for at most 200 steps and stops when the bracket is narrower than 1e-7 of the coordinate unit.

## Symbols and units

- Corners: northing and easting, in one length unit (ft, ftUS, or m).
- A: the area to cut off, in any area unit, converted to the coordinate unit squared.
- d: distance from the side to the line, in the coordinate unit.
- Results: areas in the coordinate unit squared; the line's bearing (quadrant, DMS) and length; each end as coordinates and as side, station, and offset.

## Domain

The figure must have 3 to 1,000 corners in order, straight sides, and no sides that cross or touch. A curve call is refused with `DEGENERATE_GEOMETRY`. The required area must be more than zero and less than the whole figure; otherwise the result is `NO_SOLUTION`, naming the figure's area.

The line must make exactly two pieces. A parallel or bearing line that would cross the boundary more than twice is refused. So is a through-point line that would leave the figure.

## Approximations

None on the plane beyond the 1e-7 stopping width. Areas are grid areas. For ground area, divide by the square of the combined factor.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: A 100 ft square cut parallel to its south side
- sourceEdition: 2026-10-20
- sourceLocator: Base 100 ft, sides at 90°: A(d) = 100·d, so 2,000 ft² gives d = 20 ft and a line 100 ft long
- independent: no (the textbook partition examples in Ghilani & Wolf, ch. 12, are not open and are not yet reproduced)
- inputs: corners (0, 0), (0, 100), (100, 100), (100, 0) as northing, easting in ft; area 2,000 ft²; parallel to side 1
- outputs: offset 20 ft, length 100 ft, cut 2,000 ft², rest 8,000 ft²
- tolerance: 1e-7 ft
- verifiedBy: the tool's primary example and `trapezoid_matches_the_quadratic`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_area_partition.py`: a separate solution with Shapely (GEOS) half-plane intersections and line substrings, solved with SciPy's brentq. It covers 18 random star-shaped figures with 3 to 8 corners, all three cases, on state-plane-sized coordinates. Lengths and offsets agree within 1e-5 ft, and areas within 5e-4 ft².
- `core/vectors/survey.cogo.area-partition.jsonl`: those vectors plus four refusals (area too large, curve call, self-crossing figure, missing bearing), run through the core on every build.

## Invariants

- `core/crates/gp-survey/tests/partition.rs` `area_partition_invariants`: on 1,000 random star-shaped figures, across all three cases:
  - the two pieces add to the whole within 1e-9 relative;
  - the cut equals the required area within 1e-6 of the line length;
  - both ends lie on the boundary within 1e-9 of the figure's size;
  - the only refusals are `DEGENERATE_GEOMETRY`.
- `trapezoid_matches_the_quadratic`: a 45°/90° trapezoid matches the closed-form root.
