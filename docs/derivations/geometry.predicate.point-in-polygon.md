<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Point in polygon (`geometry.predicate.point-in-polygon`)

## Method

Stand at the point and watch the boundary go by. Walk the outline edge by edge and add up the turn in the direction you are looking; if the outline encloses you, you will have turned through a full circle by the end, and if it does not, the turns will cancel. That total, divided by 360°, is the winding number, and it is computed here from geodesic azimuths taken from the point to each vertex, so the edges are geodesics on the ellipsoid rather than straight lines on a map. Holes are walked the opposite way round so they subtract.

Two rules read that number differently and both are reported. The winding rule calls a point inside when the number is anything but zero; the even-odd rule calls it inside when the number is odd. For a simple outline they agree. For one that crosses itself they need not, and reporting both says so rather than silently picking. A point within a millimetre of an edge is called on the boundary instead of either.

## Equations

- Winding number: w = (1/360°) Σ over edges of the signed change in azimuth from the point to the edge's start and end, the azimuths from the geodesic inverse problem.
- Winding rule: inside when w ≠ 0. Even-odd rule: inside when w is odd.
- Holes: reversed before summing, so an enclosed hole contributes −1.
- On the boundary: the geodesic distance from the point to the nearest edge is under 1 mm.
- Reported distance: the least geodesic distance from the point to any edge.

## Symbols and units

The polygon and the points are latitude and longitude in degrees, holes by ring. `winding` is the winding number as an integer; `nonzero` and `even_odd` are the two verdicts; `distance` is to the nearest edge in the chosen length unit; `inside_count` counts the points the winding rule calls inside, and `first` is the first point's verdict.

## Domain

Any polygon with holes, and any number of points, as long as each ring is smaller than a hemisphere seen from the point being tested — beyond that the azimuths no longer sum the way the argument requires. Rings may cross the antimeridian and may enclose a pole, since only differences of azimuth are used and no longitude is ever compared against another.

## Approximations

None in the rule: the winding number is an integer and comes out as one. The one threshold is the millimetre that decides on-boundary, which exists because a point exactly on an edge has no correct answer and a point a nanometre away has one nobody should rely on. Distances are geodesic and exact.

## Worked example

- sourcePublisher: the GEOS contributors, with PROJ
- sourceTitle: GEOS through shapely, on PROJ's azimuthal equidistant plane
- sourceEdition: GEOS 3.11.4, shapely 2.0.7, PROJ 9.3.0
- sourceLocator: `Polygon.contains(Point)` and `Polygon.exterior.distance(Point)` on `+proj=aeqd +ellps=WGS84` coordinates
- independent: yes
- inputs: 51 points against a square, an L and a triangle — some placed by hand inside, outside and in the notch, and the rest scattered at random over each shape's box
- outputs: each point's verdict under both rules, and its distance to the nearest edge
- tolerance: exact on the verdicts; 5e-5 relative on the distances
- verifiedBy: golden vectors v009 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The reference is not a second implementation of the same idea. GEOS runs an ordinary planar point-in-polygon test on projected coordinates; the core sums geodesic azimuths on the ellipsoid. Different algorithm, different surface, and for shapes this size they must still agree on every point that is not sitting on a boundary. Over all 51 points they do, under both rules, with no exceptions. The distances — GEOS measuring in the plane, the core along the geodesic — agree to 7.4e-6 relative at worst.

No point is placed within a metre of an edge, and that is deliberate rather than convenient. Within a millimetre the tool reports on-boundary, which is a third verdict this reference has no way to express; between a millimetre and a metre the two methods are entitled to differ by the projection's own error, so a vector there would be pinning the projection rather than the rule.

A flipped verdict was fed to the runner before the vectors were kept, to confirm it actually reads them: it failed, naming the vector and the point. A vector that cannot fail is worse than no vector.

## Differential tests

- `tools/vectors/gen_pip_geos.py`: fourteen vectors covering 51 points from GEOS, appended to the frozen file
- `core/crates/gp-geometry/tests/predicate.rs` `point_in_polygon_invariants`: the structural properties and the cross-tool checks
- `core/vectors/geometry.predicate.point-in-polygon.jsonl`: 22 vectors, the first eight from GeodSolve and the rest from GEOS

## Invariants

- `core/crates/gp-geometry/tests/predicate.rs` `point_in_polygon_invariants`: a polygon's own centroid, taken from `geometry.shape.centroid`, is inside it for a convex shape and its interior point is inside for any shape; every vertex of the polygon is reported as on the boundary, which is the one place the answer is known without computing anything; the two rules agree on every point of a simple outline, and `inside_count` is the number of points the winding rule called inside; reversing the winding of the polygon changes neither verdict, since inside does not depend on which way the outline was drawn; the distance reported for a point is zero at a vertex and grows as the point is moved away along a line; and a point far outside has a winding number of exactly zero rather than a small non-zero one
