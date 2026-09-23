<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Great-circle destination (`navigation.geodesic.spherical-direct`)

## Method

Where a great circle from a point on a given course reaches after a given distance, on a **sphere** of mean radius 6,371.008771 km. The companion to `spherical-inverse`, and for the same reason: to reproduce what spherical software and published examples give.

With δ = distance / R the destination is

φ₂ = asin(sin φ₁ cos δ + cos φ₁ sin δ cos θ)
λ₂ = λ₁ + atan2(sin θ sin δ cos φ₁, cos δ − sin φ₁ sin φ₂)

and the final course comes from solving the inverse problem backwards and adding 180°, which is more reliable than carrying the course forward through the direct formula.

The tool reports the `offset` — how far the spherical destination is from the ellipsoidal one for the same course and distance — so the error is a number rather than an assurance.

## Equations

- δ = distance / R, R = 6,371.008771 km.
- φ₂ = asin(sin φ₁ cos δ + cos φ₁ sin δ cos θ).
- λ₂ = λ₁ + atan2(sin θ sin δ cos φ₁, cos δ − sin φ₁ sin φ₂), wrapped to (−180, 180].
- Final course = the reverse initial course + 180°.

## Symbols and units

`lat1`, `lon1`, `course` and `distance`, with an optional `radius`. Out come `lat2`, `lon2`, the `final_course` and the `offset` from the ellipsoidal answer.

## Domain

Any start, course and distance, including distances over a half circumference, which wrap round the far side.

## Approximations

The sphere, as above. Over a transatlantic distance the destination is some kilometres from where the ellipsoidal calculation puts it, and the `offset` says how many.

## Worked example

- sourcePublisher: the great-circle formulas
- sourceTitle: the spherical direct formulas for destination point
- sourceEdition: definition
- sourceLocator: phi2 = asin(sin phi1 cos delta + cos phi1 sin delta cos theta)
- independent: yes
- inputs: 21 starts, including due north, due east along the equator, a course from 89° north, distances from 50 km to 10,000 km, and all four quadrants of course
- outputs: the destination and the final course
- tolerance: 1e-9° on position, 1e-8° on the course
- verifiedBy: golden vectors v001 to v021, run by the core on every build
- verifiedOn: 2026-09-23

As with the inverse, GeographicLib is the wrong reference here: it answers a different question on purpose. The formulas are transcribed from their own definitions, and the round trip against `spherical-inverse` is what ties the pair together.

## Differential tests

- `tools/vectors/gen_nav_sphere_vincenty.py`: 16 of the 21 vectors, from the formulas
- `core/crates/gp-navigation/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/navigation.geodesic.spherical-direct.jsonl`: 21 starts across the globe

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `spherical_direct_invariants`: the direct and inverse problems undo each other — going out on a course for a distance and solving back gives that course and that distance, to a nanodegree; due north from the equator for R·π/2 lands on the pole; due east along the equator stays on the equator; a course of 90° from the equator for a quarter circumference ends a quarter turn of longitude away; going a full circumference returns to the start; and the offset from the ellipsoidal answer grows with distance, since it is an accumulating difference rather than a fixed one
