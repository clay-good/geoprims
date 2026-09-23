<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Vincenty direct (`navigation.geodesic.vincenty-direct`)

## Method

The destination from a point, an azimuth and a distance, by **Vincenty's 1975 direct method**. The companion to `vincenty-inverse`, for the same purpose: reproducing what the software and the published tables of the last forty years give.

The direct problem does not need to iterate on longitude, only on σ, the arc on the auxiliary sphere. Starting from σ = s/(b A) it applies the Δσ correction and repeats until σ settles to 1e-12 radians — usually three or four passes. The latitude comes from a ratio whose denominator uses (1 − f) to undo the reduction, and the longitude from λ with the same C correction the inverse uses.

`karney_difference` is reported here too: how far the Vincenty destination is from the exact one, in metres.

## Equations

- U₁ = atan((1 − f) tan φ₁); σ₁ = atan2(tan U₁, cos α₁); sin α = cos U₁ sin α₁.
- u², A, B as in the inverse.
- Iterate σ = s/(bA) + Δσ(σ) to 1e-12 rad.
- φ₂ = atan2(sin U₁ cos σ + cos U₁ sin σ cos α₁, (1 − f)·√(sin²α + (sin U₁ sin σ − cos U₁ cos σ cos α₁)²)).
- λ = atan2(sin σ sin α₁, cos U₁ cos σ − sin U₁ sin σ cos α₁), then L with the C correction.

## Symbols and units

`lat1`, `lon1`, `azimuth`, `distance`, with an optional `ellipsoid` or explicit `a` and `inverse_flattening`. Out come `lat2`, `lon2`, `azimuth2` and `karney_difference`.

## Domain

Any start, azimuth and distance. Unlike the inverse, the direct method has no convergence problem: there is nothing antipodal about going a given way for a given distance.

## Approximations

The same truncated series as the inverse, about a millimetre on a long line, reported as `karney_difference`.

## Worked example

- sourcePublisher: T. Vincenty, Directorate of Overseas Surveys
- sourceTitle: Direct and Inverse Solutions of Geodesics on the Ellipsoid with Application of Nested Equations
- sourceEdition: Survey Review XXIII, no. 176, April 1975
- sourceLocator: the direct method's nested equations, iterated on sigma
- independent: yes
- inputs: 21 starts, including due north, due east along the equator, a course from 89° north, distances from 50 km to 10,000 km, and all four quadrants of azimuth
- outputs: the destination and the final azimuth
- tolerance: 1e-9° on position, 1e-8° on the azimuth
- verifiedBy: golden vectors v001 to v021, run by the core on every build
- verifiedOn: 2026-09-23

Written out from the paper in `tools/vectors/gen_nav_sphere_vincenty.py`, the same way as the inverse. Both were checked against the core on the first run with no adjustment, which is the outcome an independent transcription should have and does not always.

## Differential tests

- `tools/vectors/gen_nav_sphere_vincenty.py`: 16 of the 21 vectors, from the 1975 paper
- `core/crates/gp-navigation/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/navigation.geodesic.vincenty-direct.jsonl`: 21 starts across the globe

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `vincenty_direct_invariants`: the direct and inverse methods undo each other — out on an azimuth for a distance, then back by the inverse, returns that azimuth and that distance; due north from the equator for a quarter of the meridian reaches the pole; due east along the equator stays on the equator, where the ellipsoid's equator is a circle and the two methods must agree exactly; the destination agrees with Karney's to under a millimetre; and the reported `karney_difference` is that distance and not something else
