<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Cross-track and along-track distance (`navigation.route.cross-track`)

## Method

Find the point on the geodesic A→B nearest P (the foot point), then report the geodesic distance from P to that foot (signed: positive right of the course) and the distance from A along the line to it. The foot comes from Karney's interception method: repeatedly project A, B, and P gnomonically about the current guess, solve the planar perpendicular-foot problem, and map back; each step squares the error, so a few iterations reach millimeters. A spherical method is offered too: the classic formulas on the mean radius, as navigators and the Aviation Formulary use them.

## Equations

- Ellipsoidal: iterate x ← gnomonic foot of P on the line through the projections of A and B about x, until the move is under 1e-3 m.
- Spherical: XTD = asin(sin(d_AP) sin(θ_AP − θ_AB)), ATD = acos(cos(d_AP)/cos(XTD)), with d and θ the great-circle distance and initial course.
- Sign: positive when P lies clockwise (right) of the course from A to B.

## Symbols and units

φ, λ the latitudes and longitudes of A, B, and P (degrees, WGS 84); XTD the cross-track distance and ATD the along-track distance (meters, shown in the chosen unit); the segment length is A to B.

## Domain

Any three points on any cataloged or custom ellipsoid. The foot may lie beyond either end, and `within` says so; the along-track distance is then negative or longer than the segment. For P near the antipode of the line the foot is ill-conditioned, and the geodesic itself is ambiguous.

## Approximations

Karney's geodesic (2013) to nanometer accuracy, and the interception iteration to under a millimeter. The spherical method uses the mean radius 6,371.0088 km, so it differs from the ellipsoidal answer by a few tenths of a percent.

## Worked example

- sourcePublisher: Ed Williams
- sourceTitle: Aviation Formulary, "Cross track error" worked example
- sourceEdition: version 1.47
- sourceLocator: Enroute LAX (33°57′N, 118°24′W) to JFK (40°38′N, 73°47′W), at D (N34:30, W116:30): xtd = 7.4512 nm right of course, atd = 99.588 nm along course
- independent: yes
- inputs: LAX to JFK with the point 34.5, −116.5, spherical method
- outputs: 7.4512 nm right of course, 99.588 nm along it
- tolerance: 0.01 nm and 0.08 nm: the formulary takes a nautical mile as one minute of arc (R = 6,366.7 km), 0.07% smaller than the mean radius, which alone accounts for 0.07 nm of the along-track distance
- verifiedBy: golden vector v023, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-navigation/tests/navigation.rs` `cross_track_matches_a_brute_force_search`: 200 random lines, with the point before, along, and past the line, against a golden-section search for the nearest point using Karney's geodesic (geographiclib-rs)
- `tools/vectors/gen_route.py`: 22 more vectors from a perpendicularity bisection in Karney's Python geographiclib, independent of the gnomonic method
- `core/vectors/navigation.route.cross-track.jsonl`: 23 vectors, including feet beyond both ends

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `cross_track_invariants`: both ends and the foot point lie on the line (no cross-track distance), the foot keeps the same along-track distance, and `within` agrees with the along-track distance
