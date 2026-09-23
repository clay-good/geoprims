<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Bounding box, antimeridian-aware (`geometry.shape.bbox`)

## Method

Latitude and longitude behave differently here and are found differently.

Latitude is an interval, and the difficulty is that a geodesic does not stay between the latitudes of its ends: it bows toward the nearer pole, and on a long mid-latitude edge it bows a long way. So the box takes the endpoints and, for every edge, the latitude of that edge's vertex — the point where the path is heading due east or due west — found by bisecting the azimuth along the geodesic. A line from 60° N, 60° W to 60° N, 60° E reaches 73.9° N in the middle, which the endpoints give no hint of.

Longitude is a circle, and on a circle there is no smallest and largest. What there is, is a largest gap: mark every point and every edge's arc of longitude, find the widest stretch nothing covers, and the box is everything else. That gives the shortest box that holds the shape, which for something either side of the antimeridian is the short way across it rather than the long way around the world, written with west greater than east as RFC 7946 §5.2 prescribes.

## Equations

- Latitude vertex of an edge: the s along the geodesic where azimuth(s) = ±90°, by bisection on the direct problem.
- Box latitudes: south = min over endpoints and vertices, north = max over the same.
- Longitude arcs: each point a zero-length arc at its longitude; each edge the shorter arc between its ends, Δλ = ((λ₂ − λ₁ + 180) mod 360) − 180.
- Box longitudes: merge the arcs on the circle, take the widest uncovered gap, and the box is its complement: west = the gap's east end, east = the gap's west end, span = 360 − gap.
- A polygon whose outline winds a pole covers every longitude and reaches that pole.

## Symbols and units

λ and φ are longitude and latitude in degrees; `west`, `south`, `east`, `north` are the box, with `west` greater than `east` when it crosses the antimeridian; `lon_span` is the width in degrees going east from west, always between 0 and 360. `crosses_antimeridian` and `pole` report the two cases that make the box unusual.

## Domain

Any number of points, taken as loose points, as a line, or as a polygon — and which of those is asked for changes the answer, since only a line or polygon has edges to bow. The whole sphere is in range, including the poles and the antimeridian.

## Approximations

The bisection for each edge's latitude vertex, which converges to the last bit a double holds and is stated at 1e-9°. Nothing else is approximate. There is one place the answer is not unique rather than not accurate: where several gaps in longitude are exactly the widest, the rule has nothing to choose between them, and two correct implementations can return different boxes of the same width.

## Worked example

- sourcePublisher: Charles F. F. Karney and the geographiclib contributors; IETF
- sourceTitle: geographiclib's geodesic direct problem; RFC 7946, The GeoJSON Format
- sourceEdition: geographiclib 2.1; RFC 7946
- sourceLocator: `Geodesic.Line(...).Position(s)["azi2"]` bisected for ±90°; RFC 7946 §5.2 on antimeridian-crossing boxes
- independent: yes
- inputs: ten shapes — long lines that bulge, a transatlantic line, triangles and quadrilaterals, a box and loose points across the antimeridian, a narrow equatorial strip, and a high-latitude quadrilateral
- outputs: west, south, east, north and the longitude span of each
- tolerance: 1e-9°, about 0.1 mm
- verifiedBy: golden vectors v013 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The core finds each edge's latitude vertex from Clairaut's relation; the reference finds it by asking Karney's geographiclib for the azimuth at a point and bisecting until it passes due east or due west. Same condition, different library, different method. On the line from 60° N, 60° W to 60° N, 60° E the bisection lands on 73.90894412305497° with an azimuth of 89.99999999999997° — the tool's figure in every digit a double carries. Across all ten shapes and all five outputs, the worst disagreement is zero.

The longitude rule is taken from the RFC's text rather than from the core, and one case had to be changed rather than recorded. Points at 170° W, 60° W, 50° E and 160° E leave three gaps of exactly 110°: the rule says take the largest, there are three, and the two implementations picked different 250° boxes. Both are right. A vector pinning either would be pinning a coin toss, so the case was moved to longitudes with a single widest gap and the ambiguity written into the limitations instead.

## Differential tests

- `tools/vectors/gen_bbox_geodesic.py`: ten vectors from geographiclib and RFC 7946, appended to the frozen file
- `core/crates/gp-geometry/tests/envelope.rs` `bbox_invariants`: the containment property, the edge bulge, the antimeridian, and the shape modes
- `core/vectors/geometry.shape.bbox.jsonl`: 22 vectors, the first twelve hand-checked and the rest from the reference above

## Invariants

- `core/crates/gp-geometry/tests/envelope.rs` `bbox_invariants`: every input point lies inside the box it produced, which is the one thing a bounding box must never get wrong, checked with the crossing case where a naive comparison fails; a line's box contains the same points' box and is never smaller, since edges can only add; the bulge is real and positive — the box of the 60° N line reaches above 73° while the box of the same two points does not leave 60°; the longitude span is what going east from west to east gives, never negative and never above 360; a box crossing the antimeridian reports west greater than east and raises `CROSSES_ANTIMERIDIAN`, while one that does not reports west less than east and stays quiet; and reversing the order of the points leaves the box unchanged
