<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Douglas-Peucker simplification (`geometry.simplify.rdp`)

## Method

Take the straight line from the first vertex to the last, and find the vertex furthest from it. If that vertex is closer than the tolerance, every vertex between the ends can go and the line stands for all of them. If it is not, that vertex has to stay, and the problem splits in two at it: the same question is asked again of each half. The recursion ends when no remaining piece has a vertex further out than the tolerance.

Two things make the geodesic version more than that. The recursion runs on an azimuthal equidistant plane centred on the shape, where distances are honest near the centre, but the guarantee is then checked on the ellipsoid: any edge that turns out to have a dropped vertex further than the tolerance by geodesic distance is split again. And a polygon has no natural first and last vertex, so it is cut at the vertex furthest from its first before the recursion and rejoined afterwards. With topology preserved, any edge that ends up crossing another gets its furthest vertex back, repeatedly, until none cross.

## Equations

- Perpendicular distance from vertex v to the segment from a to b, on the plane; the segment, not the infinite line, so a vertex beyond an end measures to that end.
- Keep v and recurse on (a, v) and (v, b) when that distance exceeds the tolerance; otherwise drop everything strictly between a and b.
- Checked afterwards on the ellipsoid: geodesic distance from each dropped vertex to its simplified edge, against the same tolerance.
- `max_deviation` is the largest of those geodesic distances over the whole result.

## Symbols and units

The points are latitude and longitude in degrees; the tolerance is a distance on the ground in the chosen unit. `vertices_in` and `vertices_out` count the input and the result, `restored` counts vertices put back to stop edges crossing, and `max_deviation` is the furthest any dropped vertex ended up from the line that replaced it.

## Domain

Lines and polygons of any length, over shapes up to a few hundred kilometres across, where the plane the recursion runs on is faithful. The tolerance must be greater than zero; zero is refused rather than read as keeping everything. Two points cannot be simplified further and come back unchanged.

## Approximations

The recursion is exact for the plane it runs on, and the guarantee it offers is then verified on the ellipsoid rather than assumed, which is why an edge can be split a second time after the planar pass. The crossing check, when topology is preserved, is done on the plane and is exact for shapes at these sizes. Nothing is approximate about which vertices are kept: that is a decision, and it is either right or wrong.

## Worked example

- sourcePublisher: the GEOS contributors
- sourceTitle: GEOS through shapely, `LineString.simplify`
- sourceEdition: GEOS 3.11.4, shapely 2.0.7, PROJ 9.3.0
- sourceLocator: `LineString.simplify(tolerance, preserve_topology=False)` on `+proj=aeqd +ellps=WGS84` coordinates
- independent: yes
- inputs: six shapes — a gentle curve, a zigzag, a near-straight line, a sharp dogleg, a long wandering track and a high-latitude track — each at tolerances of 5, 25, 100 and 400 m
- outputs: the number of vertices kept and the furthest a dropped vertex ended up from the result
- tolerance: exact on the vertex counts; 1e-6 relative on the deviation
- verifiedBy: golden vectors v006 to v029, run by the core on every build
- verifiedOn: 2026-09-23

What can go wrong in Douglas-Peucker is not arithmetic but *which vertices survive*. An off-by-one in the recursion, or measuring to the infinite line rather than to the segment, changes the answer at some tolerances and not at others — which is exactly the kind of fault a single worked example misses. So the vectors pin the kept-vertex count over a grid, and GEOS's own implementation of the same rule decides what it should be. Across all twenty-four combinations the two never chose differently, and the deviations agree to 1.3e-7 relative.

The grid is chosen so that each shape passes through several regimes. The zigzag keeps all fourteen of its vertices at 5 m and at 25 m, drops two at 100 m, and collapses to two at 400 m once the tolerance exceeds its amplitude. The near-straight line collapses to two points at every tolerance including the smallest, since it was already within 2.07 m of straight — a reminder that the tolerance is a bound and not a target.

## Differential tests

- `tools/vectors/gen_rdp_geos.py`: twenty-four vectors from GEOS over six shapes and four tolerances, appended to the frozen file
- `core/crates/gp-geometry/tests/simplify.rs` `rdp_invariants`: the guarantee, the monotonicity, and the subset property
- `core/vectors/geometry.simplify.rdp.jsonl`: 29 vectors, the first five from the original Douglas-Peucker and Visvalingam-Whyatt implementations and the rest from GEOS

## Invariants

- `core/crates/gp-geometry/tests/simplify.rs` `rdp_invariants`: the reported deviation never exceeds the tolerance asked for, which is the promise the tool makes and the only one that matters; the result is always a subset of the input in order — vertices are dropped, never moved or invented — checked vertex by vertex rather than by counting; the first and last points always survive; raising the tolerance never increases the number of vertices kept, across a sweep of tolerances on one shape; a tolerance of zero is refused rather than silently read as keeping everything, since a simplification that simplifies nothing is a request that went wrong somewhere; a two-point line is returned unchanged at any tolerance; and preserving topology never returns fewer vertices than not preserving it
