<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Intermediate point (`navigation.geodesic.intermediate-point`)

## Method

The point a given fraction of the way from one place to another — a quarter of the way, halfway, nine tenths — reported two ways, because the two answers differ and the difference is worth seeing.

The **ellipsoidal** answer is the geodesic one: solve the inverse problem for the azimuth and length, then the direct problem at that fraction of the length. It is the point you would actually reach.

The **spherical** answer is the one most code uses, and the tool reports it beside the first so the gap is visible. It is spherical linear interpolation of the two unit vectors: with δ the central angle between them, A = sin((1 − f)δ)/sin δ and B = sin(fδ)/sin δ, and the interpolated direction is A·p₁ + B·p₂. That is exact on a sphere and a few kilometres out on a long ellipsoidal route.

The third output, `offset`, is the distance between the two — the size of the approximation, in metres, for this particular route and fraction.

## Equations

- δ = the central angle between the unit vectors of the two points.
- A = sin((1 − f)δ) / sin δ; B = sin(fδ) / sin δ; p = A·p₁ + B·p₂, renormalised.
- Ellipsoidal: the direct problem at distance f·s₁₂ along the initial azimuth.
- offset = the geodesic distance between the two answers.

## Symbols and units

`lat1`, `lon1`, `lat2`, `lon2` in degrees and `fraction` from 0 to 1. Out come `lat` and `lon` (spherical), `ellipsoidal_lat` and `ellipsoidal_lon`, and the `offset` between them.

## Domain

Any two points and any fraction in [0, 1]. Antipodal points are the exception: every great circle through one passes through the other, so there is no unique path and no unique intermediate point.

## Approximations

The ellipsoidal answer carries only the geodesic algorithm's error. The spherical answer is exact on a sphere and approximate on the Earth, and the tool measures that error rather than describing it — which is the point of reporting both.

## Worked example

- sourcePublisher: Charles Karney (GeographicLib)
- sourceTitle: GeographicLib's `GeodSolve`
- sourceEdition: GeographicLib 2.7
- sourceLocator: the direct problem at a fraction of the inverse problem's distance
- independent: yes
- inputs: 48 cases — six routes at seven fractions each, including both endpoints, the quarter points, the midpoint, a nearly antipodal equatorial pair, and a route across the antimeridian
- outputs: the ellipsoidal position at each fraction
- tolerance: 1e-9°
- verifiedBy: golden vectors v001 to v048, run by the core on every build
- verifiedOn: 2026-09-23

The reference takes the ellipsoidal answer from `GeodSolve` alone. The fractions 0 and 1 are in the set deliberately: they are the cases where the answer must be an endpoint exactly, and where an off-by-one in an interpolation shows up as a visible error rather than a small one.

## Differential tests

- `tools/vectors/gen_nav_geodesic.py`: 42 of the 48 vectors, from `GeodSolve`
- `core/crates/gp-navigation/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/navigation.geodesic.intermediate-point.jsonl`: 48 cases across six routes

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `intermediate_point_invariants`: fraction 0 is the first point and fraction 1 the second, exactly, in both the spherical and the ellipsoidal answer; fraction 0.5 gives the same place as `navigation.geodesic.midpoint`, so the two tools do not disagree about halfway; the distance from the start grows with the fraction, and at fraction f is f of the total, which is what "a fraction of the way" has to mean; the offset between the two answers is zero at the endpoints and largest in the middle; and on a short route the two answers agree to well under a metre, while on a transatlantic one they part by kilometres — the reason both are reported
