<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Smallest enclosing shapes (`geometry.shape.enclosing`)

## Method

Three shapes around one set of points, each found its own way.

The circle is the smallest one containing every point, by Welzl's algorithm on an azimuthal equidistant plane. That plane keeps distances and directions true from its own centre and nowhere else, so the answer is only right if the circle's centre and the plane's centre are the same point. They are made the same by iteration: solve, re-centre the plane on the result, solve again, until the centre stops moving. The fixed point of that loop is the smallest geodesic circle.

The convex hull is the great-circle hull on the sphere, by monotone chain in a gnomonic projection from the points' mean — gnomonic because it is the projection in which great circles are straight lines, so a planar hull is a spherical one. Its area is then measured on the ellipsoid.

The rectangle is the smallest-area rectangle enclosing the hull. A minimal rectangle always has a side flush with a hull edge, so only as many orientations need trying as the hull has edges.

## Equations

- Welzl: the smallest circle through at most three boundary points, found by incremental randomised construction; iterated with the plane re-centred until |centre move| < 1 mm.
- Monotone chain: sort by x then y, sweep the lower and upper hulls keeping only left turns, by the sign of the cross product.
- Rectangle: for each hull edge, rotate the hull so that edge lies along an axis, take the extent in each direction, and keep the orientation of least area.
- Hull area: the geodesic polygon area of the hull's corners on WGS 84.

## Symbols and units

The points are latitude and longitude in degrees. `circle_lat`, `circle_lon` and `circle_radius` are the smallest enclosing circle; `hull_count` the number of hull corners and `hull_area` its area; `rect_length`, `rect_width` and `rect_azimuth` the smallest rotated rectangle, the azimuth being the direction of the long side.

## Domain

Two or more points, spread over tens of kilometres, anywhere including across the antimeridian. Two points, or points that fall on one line, give a hull and rectangle that degenerate to a line with no area and no width, which is the right answer rather than a failure. Note that points evenly spaced in latitude and longitude are not collinear on the ellipsoid: three such points enclose a real, if tiny, area.

## Approximations

The circle's radius is an exact geodesic distance and its centre is iterated to a millimetre. The hull is exact: its corners are input points, chosen by the sign of a cross product, and that choice is either right or wrong rather than approximate. The rectangle is planar on the equidistant map, good to about a part in a million for spreads of tens of kilometres. Where the hull is a triangle the rectangle is not unique, which is a property of the question rather than an inaccuracy in the answer.

## Worked example

- sourcePublisher: the GEOS contributors, with geographiclib
- sourceTitle: GEOS through shapely, with the hull area from geographiclib's PolygonArea
- sourceEdition: GEOS 3.11.4, shapely 2.0.7, PROJ 9.3.0, geographiclib 2.1
- sourceLocator: `minimum_bounding_circle`, `minimum_bounding_radius`, `convex_hull` and `minimum_rotated_rectangle` on `+proj=aeqd +ellps=WGS84` coordinates
- independent: yes
- inputs: twelve point sets — a quadrilateral, a triangle, two points, three points in a line, tight and loose clusters, scatters at the equator, at 70° north and in the southern hemisphere, points either side of the antimeridian, a long thin spread, and a square
- outputs: the circle's centre and radius, the hull's corner count and area, and the rectangle's two sides
- tolerance: 1e-5 relative on the lengths and areas, 5e-5° on the circle's centre
- verifiedBy: golden vectors v010 to v021, run by the core on every build
- verifiedOn: 2026-09-23

GEOS carries its own implementations of all three, so each output has a second opinion that shares no code with the first. The hull corner count — the part that is a decision rather than a measurement — matches on every one of the twelve. The circle's radius agrees to 2.5e-8 relative and the hull's area to 5.7e-7. The circle's *centre* is the loosest at 7.2e-6°, which is 0.8 m over a 20 km scatter: two runs of Welzl's algorithm converging from different starting points, and well inside the millimetre-per-iteration the tool claims for its own convergence, since the centre of a large circle is poorly determined by comparison with its radius.

The triangle is the interesting case and its rectangle is deliberately not pinned. GEOS returns 9874.70 × 13721.64 m and the core returns 10774.74 × 12575.44 m, which looks like a 9% disagreement. Enumerating all three edge-flush rectangles settles it: their areas are 135497137.846 m² to the metre, all three, identically. That is a theorem about triangles — every minimal enclosing rectangle of a triangle has exactly twice its area — so all three orientations are minimal and there is nothing to choose between them. Pinning either implementation's choice would pin a coin toss, so the vectors pin the circle and the hull there and the invariant test checks the property that does hold.

## Differential tests

- `tools/vectors/gen_enclosing_geos.py`: twelve vectors from GEOS, appended to the frozen file, skipping the rectangle where the hull is a triangle
- `core/crates/gp-geometry/tests/envelope.rs` `enclosing_invariants`: containment, the degenerate cases, and the triangle theorem
- `core/vectors/geometry.shape.enclosing.jsonl`: 21 vectors, the first nine from the original scenarios and the rest from GEOS

## Invariants

- `core/crates/gp-geometry/tests/envelope.rs` `enclosing_invariants`: every input point lies within the enclosing circle, measured by `navigation.geodesic.inverse` rather than by anything this tool computed, and at least one point lies on it — a circle with room to spare is not the smallest; the hull has between two and as many corners as there are points, and every hull corner is one of the input points rather than a new position; the hull's area is the one `geometry.area.polygon` gives for those corners; the rectangle is at least as large as the hull and no larger than the circle's bounding square; where the hull is a triangle the rectangle's area is exactly twice the hull's, whichever of the three minimal orientations was returned; and two points give a hull of two corners with no area and a rectangle whose short side is about 15 picometres, held at a micrometre rather than at zero, since subtracting two nearly equal projected coordinates does not leave exactly nothing
