<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Buffer a point, line, or polygon (`geometry.buffer.geodesic`)

## Method

A buffer is the set of points within a distance of the input, and that set is the union of simple pieces: a rectangle alongside every edge, something at every corner, and a cap at every open end of a line. The pieces are built on an azimuthal equidistant plane placed at the input's center, where distances from that center are true, and unioned by keeping only the boundary edges that no other piece covers. That gives the shape. It does not yet give the distances, because a plane is not the ellipsoid, so every vertex on a round or straight part is then moved: the nearest input point is found and the vertex is placed exactly the buffer distance from it by the geodesic direct problem. The result is finally measured — at every vertex and at every edge midpoint — and the worst departure is reported with the answer rather than assumed.

## Equations

- The buffer of a set S at distance d: { p : min over q in S of geodesic(p, q) = d } as a boundary, the union of the pieces below.
- Round corner of interior angle θ: an arc of radius d, so the boundary stays at d throughout.
- Mitre corner: the two offset edges extended to meet, at d / cos(θ/2) from the corner — farther out than d, which is what a mitre is. Beyond the mitre limit the corner is beveled instead.
- Corner count on a round part: enough that the chord between vertices sags less than the stated tolerance, the sag of a chord subtending Δ being d(1 − cos(Δ/2)).
- Steiner, for a convex polygon in the plane: area(buffer) = A + P·d + π d², which the vectors use as a closed form.

## Symbols and units

The vertices are latitude and longitude in degrees on WGS 84; d is the buffer distance in meters, negative to shrink a polygon; θ is a corner's interior angle. Area is reported in the chosen area unit, perimeter and the deviation in the chosen length unit. `max_deviation` is the largest measured departure from d over all vertices and edge midpoints, and `tolerance` is the budget it is held to.

## Domain

One to 5,000 vertices, as a point, a line, or a polygon with holes by ring. The input and the resulting buffer must fit within 1,000 km of their common center: the azimuthal equidistant plane the pieces are built on is faithful near its center and not far from it, and a shape beyond that is refused rather than distorted. A negative distance shrinks a polygon and may collapse it to nothing or split it into pieces, both of which are reported. A self-touching or repeated input should be made valid first, since the union is a planar operation.

## Approximations

Two, and the tool measures both rather than asserting them. The boundary of a round part is a polygon, so between vertices it is a chord inside the true curve; the vertices themselves are placed exactly. The union is computed in the plane while the vertices are placed on the ellipsoid, which is why the measurement is repeated afterwards and the worst departure travels with the answer. Mitre and square corners are not approximations at all: they sit at d / cos(θ/2) by construction, farther out than d on purpose.

## Worked example

- sourcePublisher: Charles F. F. Karney and the GeographicLib contributors
- sourceTitle: GeodSolve, the inverse and direct geodesic problems
- sourceEdition: GeographicLib 2.7
- sourceLocator: `GeodSolve -p 12` to sample 2,000 points along each of the rectangle's four edges, and `GeodSolve -i -p 12` from every boundary vertex and every chord midpoint to all 8,004 of them
- independent: yes
- inputs: the rectangle 40 −105, 40 −104.99, 40.006 −104.99, 40.006 −105 buffered by 500 m
- outputs: 157 boundary vertices, area 2.8741269858 km², perimeter 6.1815548477 km, max_deviation 0.1241033677 m
- tolerance: 1 µm on the vertices, 1 µm on the reported deviation
- verifiedBy: golden vector v030, run by the core on every build
- verifiedOn: 2026-09-23

The check does not compute a buffer. It takes the boundary the tool returned, samples the input densely with GeodSolve's direct problem, and asks GeodSolve's inverse problem how far each boundary point actually is from the input. All 157 vertices come back at 500 m within 61 nanometres, the median within 23 nanometres — so the placement by the direct problem is doing exactly what the model says.

The chord midpoints are the second half, and they explain the tool's own number. Measured the same way, the worst midpoint sags 0.124103 m inside the true buffer and the smallest sags nothing, which matches the 0.1241033677 m the tool reports as `max_deviation`. That figure is therefore not an error in placing the boundary but the depth of the polygon's chords below the curve, and an independent measurement agrees with it to the micrometer GeodSolve prints. It is well inside the 0.5 m tolerance the tool holds itself to.

## Differential tests

- `core/crates/gp-geometry/tests/buffer.rs` `geofence_buffer_is_500_m_within_half_a_meter`: every boundary vertex and edge midpoint measured back to the input with geographiclib-rs, independent of the azimuthal-equidistant construction
- `core/crates/gp-geometry/tests/buffer.rs` `a_point_buffer_is_a_geodesic_circle_even_across_the_antimeridian_and_at_a_pole`, `a_line_with_flat_ends_and_a_long_line`, `mitre_corners_respect_the_limit`: the same measurement for points, lines, and mitred corners, including a buffer spanning the antimeridian and one over a pole
- `tools/vectors/gen_buffer.py`: 23 vectors from Steiner's formula and plane closed forms on small near-equator shapes
- `core/vectors/geometry.buffer.geodesic.jsonl`: 30 vectors, the last from the GeodSolve measurement above

## Invariants

- `core/crates/gp-geometry/tests/buffer.rs` `buffer_invariants`: a buffered point is a disk, its area and perimeter always just under π d² and 2π d and within 0.2% and 0.05% of them, always under because the boundary is inscribed; the buffer of a polygon is within 0.01% of Steiner's A + Pd + πd², with A and P taken from `geometry.area.polygon` rather than computed here, at three distances; `max_deviation` never exceeds the reported tolerance; a mitre corner sits farther out than a round one on the same input at the same distance; and a mitre buffer outward by d and back inward by d returns the original area to a part in 100 million, which is the check that the outward and inward constructions are inverses

A round buffer does not round-trip that way, and the reason is worth stating because it looks like a failure. Its corners are polygons inscribed in arcs, so a corner's own inradius is short of d by the chord sag; eroding by more than d minus that sag empties the corners, and the result collapses. For the worked example, with a sag of 0.124 m, the shrink still works at −199.5 m and is empty by −199.9 m, which is where the arithmetic says it should turn over. The mitre corner has no sag, which is why it is the one that comes back exactly.
