<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Where two courses cross (`navigation.route.course-intersection`)

## Method

Each course is a line on the ellipsoid that starts at a known position and heads off in a known direction: a geodesic (the shortest path, which a great-circle route follows) or a rhumb line (a constant course, as a compass heading is flown). The crossing is where the two lines meet, and the answer is the point together with how far each course has to be run to reach it.

For geodesics the tool guesses first on a sphere, where two great circles meet at two opposite points found from a cross product, and then corrects the guess on the ellipsoid. It runs each course its current distance with the direct geodesic problem, measures the gap between the two ends with the inverse problem, and solves for the change in the two distances that closes that gap, using the direction each line is heading at its end (Newton's method). A few steps close it to under a nanometer.

For rhumb lines no iteration is needed: on a Mercator map both are straight lines, so the crossing is the meeting of two straight lines in longitude and isometric latitude, and the runs come from the rhumb inverse problem.

Two courses cross again and again as they wind round the Earth. The two nearest crossings lie about half the world apart. Reading each run within half a circumference either way, the tool reports the one that lies behind fewer of the two positions, and between equals the nearer. A crossing behind a position comes back with a negative run.

## Equations

- Spherical seed: n₁ = p₁ × t₁ and n₂ = p₂ × t₂ are the poles of the two great circles (p the position, t the direction of travel, as unit vectors); they cross at ±(n₁ × n₂)/|n₁ × n₂|, and each run is R·atan2(x·t, x·p).
- Newton step on the ellipsoid: with the ends X₁(s₁) and X₂(s₂), the gap r from X₂ to X₁ in the plane tangent at X₂ (from the inverse problem's distance and azimuth), and u₁, u₂ the unit directions of travel there, solve u₁ Δs₁ − u₂ Δs₂ = −r; stop at a gap under 1 nm (or rounding at 10,000 km).
- Rhumb: ψ = asinh(tan φ) − e·atanh(e sin φ); each course is (λ, ψ) = (λ₀, ψ₀) + u(sin α, cos α); the crossing solves the 2 × 2 linear system, trying the nearby windings in longitude, and the runs are the rhumb distances, signed by whether the crossing lies ahead.

## Symbols and units

Inputs: the two positions (`lat1`, `lon1`, `lat2`, `lon2`, degrees), the two courses (`course1`, `course2`, degrees true), `method` (geodesic or rhumb), and an optional ellipsoid (WGS 84 by default). Outputs: `distance1` and `distance2`, the runs along each course in meters (negative when behind), and the crossing's `lat` and `lon` in degrees.

## Domain

Any two positions off the poles (a course from a pole has no direction) and any two courses. Courses along the same line, or parallel, have no crossing and are refused with NO_SOLUTION. The ellipsoid may be any in the catalog or a custom one.

## Approximations

None in the answer: the geodesic crossing closes to under a nanometer, and the rhumb crossing is exact to floating point. The choice between the two nearest crossings is a convention, stated above; where a crossing lies almost exactly half the world away along a course, ahead and behind name the same place and the choice can go either way.

## Worked example

- sourcePublisher: C. F. F. Karney, GeographicLib
- sourceTitle: IntersectTool (the C++ implementation of Karney's geodesic intersection algorithm)
- sourceEdition: GeographicLib 2.7
- sourceLocator: `IntersectTool -c -R 40100000`, every crossing within one circumference, the two nearest kept and the tool's rule applied
- independent: yes
- inputs: 500 random pairs of positions up to about 3,000 km apart with random courses, anywhere from 80° S to 80° N, including pairs across the antimeridian
- outputs: the run along each course to the crossing
- tolerance: 10 µm on each run
- verifiedBy: `core/crates/gp-navigation/tests/course_intersection.rs` `course_intersection_matches_intersect_tool`; golden vectors v009 to v018
- verifiedOn: 2026-09-24

IntersectTool is GeographicLib's own C++ implementation of geodesic intersections, a different code base and a different iteration from this tool's Newton method on geographiclib-rs; what the two share is the ellipsoid. On all 500 pairs they report the same crossing, with the runs agreeing to within 10 µm. Pairs where the two nearest crossings are within 0.1% of each other in total run, or where a crossing lies within 1% of half the world away along a course, are left out of the comparison, because there either crossing is a fair answer; drawing 620 pairs to keep 500 shows how rare that is.

The rhumb crossings are checked the other way round, as the rhumb case has no second implementation to hand: carrying each course its reported run with the rhumb direct tool lands on the reported crossing within a millimeter, and the golden vectors take the Mercator crossing with RhumbSolve's runs.

## Differential tests

- `tools/vectors/gen_intersect_more.py`: the IntersectTool fixture (500 pairs) and vectors v009 to v022; `tools/vectors/gen_intersect.py` wrote v001 to v008
- `core/crates/gp-navigation/tests/course_intersection.rs` `course_intersection_matches_intersect_tool`: all 500 pairs, within 10 µm

## Invariants

- `core/crates/gp-navigation/tests/course_intersection.rs` `course_intersection_invariants`: on six pairs (the Gulf of Maine, the tropics, the Tasman Sea, a pair at 60° N, one across the antimeridian, and one in the Arctic), by both methods: swapping the two positions swaps the two runs; carrying each course its run with `navigation.geodesic.direct` or `navigation.rhumb.direct` (the reverse course for a negative run) lands within a millimeter of the reported crossing, a check through tools that do not share this tool's iteration; and two courses along the same line are refused with NO_SOLUTION.
