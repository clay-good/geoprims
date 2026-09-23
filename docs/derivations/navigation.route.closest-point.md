<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Closest point on a route (`navigation.route.closest-point`)

## Method

Walk the route one leg at a time. On each leg find the foot of the perpendicular from the position by the same gnomonic interception used by `navigation.route.cross-track`: project the leg's two ends and the position about the current guess, solve the planar foot, map back, repeat. The planar parameter says where the foot falls on the leg; outside [0, 1] the foot is off the leg and the nearer end is the closest point on it instead. Keep the leg whose closest point is nearest, and report it with the distances that were accumulated getting there: the lengths of the legs already passed plus the run from this leg's start to the closest point.

## Equations

- Foot on leg k: x ← the gnomonic foot of P on the line through the projections of Aₖ and Bₖ about x, iterated until the move is under 1e-6 m.
- Position on the leg: t = ((P − A)·(B − A)) / |B − A|², in the gnomonic plane. The closest point is the foot when 0 ≤ t ≤ 1, A when t < 0, and B when t > 1.
- Cross-track sign, from the planar cross product: (B − A) × (P − A) negative means P is right of the course, and the reported distance is positive.
- Along-route distance: s = Σ_{j<k} |AⱼBⱼ| + |AₖC|, each length a geodesic inverse.
- Route length: Σ_j |AⱼBⱼ| over every leg, including legs the answer did not choose.

## Symbols and units

Aₖ and Bₖ are the ends of leg k, P the position, C the closest point; φ and λ are latitude and longitude in degrees on the chosen ellipsoid, distances in meters and shown in the chosen unit. Legs are numbered from 1. A repeated waypoint makes a zero-length leg, which is skipped rather than counted.

## Domain

Two to 1,000 waypoints on any cataloged or custom ellipsoid. A leg a quarter of the Earth or more from the position cannot be projected gnomonically, so it is skipped: it keeps its length in the route total but cannot hold the closest point, which is right, since some other leg is nearer. Only a position that far from every leg is refused. Skipping rather than refusing matters on a long route: the first version failed on a position standing on a route's own first waypoint, because the last leg of a New York to Dubai route is a quarter of the Earth from New York. Legs across the antimeridian are ordinary — longitudes are only ever differenced, never compared — and the closest point's longitude is wrapped into [−180, 180) on the way out.

## Approximations

Karney's geodesic (2013) to nanometer accuracy, and the interception iteration to under a millimeter, so the answer is exact to the precision the outputs are shown at. No approximation is made in choosing the leg: every leg is solved, not just the ones near the position, so the answer does not depend on a search window. Where two legs are equally near, the earlier wins, which is a tie-break rather than a measurement.

## Worked example

- sourcePublisher: Charles F. F. Karney and the GeographicLib contributors
- sourceTitle: GeodSolve, the inverse geodesic problem
- sourceEdition: GeographicLib 2.7
- sourceLocator: `GeodSolve -i -p 6` on the five legs, on 40.50004326400153 −104 to 40.5 −103.9, and on 40 −104 to the closest point
- independent: yes
- inputs: the six-waypoint route 40 −105, 40 −104, 41 −104, 41 −103, 40 −103, 40 −102 with the position 40.5, −103.9
- outputs: leg 2, closest point 40.50004326400153 −104, cross-track 8.476774536 km right of course, 140.917934316 km along the route, route length 477.010065496 km
- tolerance: 1e-6 m, the precision GeodSolve was asked to print
- verifiedBy: golden vector v023, run by the core on every build
- verifiedOn: 2026-09-22

The check is the defining property rather than the tool's own arithmetic. GeodSolve gives the course from the closest point to the position as 90.00000000000 and the course along the leg there as 0.00000000000 — perpendicular to eleven decimals — and the distance between them as 8476.774536 m. The five legs measure 85393.409130, 111044.260879, 84134.725478, 111044.260879, and 85393.409130 m, summing to 477010.065496 m; the first leg plus the 55524.525186 m from the second leg's start to the closest point gives 140917.934316 m.

## Differential tests

- `core/crates/gp-navigation/tests/navigation.rs` `closest_point_matches_a_brute_force_search`: the closest point on 150 random routes of three to six waypoints, against a bisection on every leg for where the course to the position turns from ahead of the leg to behind it, using geographiclib-rs directly rather than the gnomonic method
- `tools/vectors/gen_route.py`: vectors from a perpendicularity bisection in Karney's Python geographiclib, the same reference the cross-track vectors come from
- `core/vectors/navigation.route.closest-point.jsonl`: 23 vectors, 22 from the bisection reference and one from GeodSolve

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `closest_point_invariants`: the closest point is perpendicular to its leg, measured by `navigation.geodesic.inverse` rather than by the tool; a point taken from the route returns itself with no cross-track distance and the along-route distance it was built at; the route length is the legs summed by the inverse solver; every waypoint's own along-route distance is the running total; and the answer is unchanged by reversing the route, up to the along-route distance being measured from the other end
