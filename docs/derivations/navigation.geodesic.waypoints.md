<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Waypoints along a route line (`navigation.geodesic.waypoints`)

## Method

The route between two points is solved once as an inverse geodesic problem, which gives its length and the azimuth it leaves on. Each waypoint is then placed by the direct problem: travel that azimuth from the start for a given distance, and report where you arrive. Asking for n intervals places n + 1 points, the first and last being the route's own ends; asking for a spacing places as many as fit, and asking for fractions places them where you say.

## Equations

- Inverse problem between the ends: s₁₂ and α₁.
- Waypoint k of n: the direct problem from (φ₁, λ₁) on azimuth α₁ for distance k·s₁₂/n.
- By spacing d: k runs while k·d ≤ s₁₂, with the far end added so the line is closed.
- By fraction f: distance f·s₁₂, with f from 0 to 1.

## Symbols and units

Latitudes and longitudes in degrees; the route's length in kilometres and spacing in any length unit. `count` is the number of points returned, which is one more than the number of intervals. GPX and GeoJSON of the same points are returned for loading elsewhere.

## Domain

Any two points on any ellipsoid in the registry, including across the antimeridian and over the poles. Between two nearly antipodal points the shortest path is poorly determined — many geodesics of almost the same length join them — so the line drawn there is sensitive to small changes in either end.

## Approximations

None: every point comes from Karney's direct geodesic solver, good to nanometres, rather than from interpolating between the ends. Interpolating would put the points off the line, which is the error this tool exists to avoid — on New York to London the straight-line midpoint in latitude and longitude misses the true one by several hundred kilometres.

## Worked example

- sourcePublisher: C. F. F. Karney, GeographicLib
- sourceTitle: GeodSolve, the inverse and direct geodesic problems on WGS 84
- sourceEdition: GeographicLib version 2.7
- sourceLocator: `echo "40.6413 -73.7781 51.47 -0.4543" | GeodSolve -i -p 6` gives azimuth 51.38164785837° over 5554908.790548 m; halving that and running `GeodSolve -p 8` from the same start on that azimuth gives 52.2375218022132 −41.2903387000114; both run for this note
- independent: yes
- inputs: New York (40.6413, −73.7781) to London (51.47, −0.4543) in 10 intervals
- outputs: 11 points over 5554.9087905475 km, the sixth at 52.23752180221283°, −41.290338700015006°
- tolerance: 1e-6 km on the length and 1e-8 degrees on the point, which is the precision GeodSolve was asked to print
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-22

## Differential tests

- `tools/vectors/gen_route.py`: Karney's geographiclib for Python, placing points along random routes by the direct problem
- `core/vectors/navigation.geodesic.waypoints.jsonl`: 22 vectors from that reference, run through the core on every build

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `geodesic_waypoints_invariants`: over four routes, one across the antimeridian and one spanning most of the globe, n intervals give n + 1 points and `count` agrees; the first and last points are the route's own ends; every consecutive pair is the same distance apart, measured by `navigation.geodesic.inverse` rather than by the tool itself, and those steps sum to the line's length; and walking the route the other way gives the same points in reverse
