<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Great-circle distance and course (`navigation.geodesic.spherical-inverse`)

## Method

The distance and courses between two points on a **sphere**, not on the ellipsoid. That is a deliberate choice, and it is what makes the tool useful: almost every piece of navigation software, every spreadsheet, every textbook worked example uses the spherical formulas, and when you are checking one of those you need the number it produced, not the better number.

The sphere has the mean radius of WGS 84, 6,371.008771 km. The central angle comes from the atan2 form:

σ = atan2(√((cos φ₂ sin Δλ)² + (cos φ₁ sin φ₂ − sin φ₁ cos φ₂ cos Δλ)²), sin φ₁ sin φ₂ + cos φ₁ cos φ₂ cos Δλ)

and the distance is R·σ. The atan2 form matters: the plain arccos version loses precision badly on short lines, where cos σ approaches 1 and its inverse is ill-conditioned — a few hundred metres can come out wrong by metres.

The tool reports the **ellipsoidal** distance beside its own, and the difference between them, so the cost of the spherical assumption is visible. It is about 0.3% at worst and varies with where on Earth the line is, which is why a single fudge factor does not fix it.

## Equations

- σ by the atan2 form above; distance = R σ with R = 6,371.008771 km.
- Initial course = atan2(sin Δλ cos φ₂, cos φ₁ sin φ₂ − sin φ₁ cos φ₂ cos Δλ), wrapped to [0, 360).
- Final course = the reverse initial course plus 180°.

## Symbols and units

`lat1`, `lon1`, `lat2`, `lon2` in degrees, with an optional `radius` for a different sphere. Out come the `distance`, both courses, the `ellipsoidal_distance`, and the two differences.

## Domain

Any two points. Antipodal points have no unique great circle, and coincident points no course.

## Approximations

The sphere is the approximation, and it is the point. Against the ellipsoid the distance differs by up to about 0.3%, which on a transatlantic route is some fifteen kilometres. The tool reports that difference rather than describing it.

## Worked example

- sourcePublisher: the great-circle formulas
- sourceTitle: the spherical law of cosines in its atan2 form, and the initial-course formula
- sourceEdition: definition
- sourceLocator: sigma = atan2(...); distance = R sigma with R the WGS 84 mean radius
- independent: yes
- inputs: 21 pairs, including a quarter of the equator, a meridian, transatlantic and transpacific routes, a 7 km line, and pairs in both hemispheres
- outputs: the distance and both courses
- tolerance: 1e-9
- verifiedBy: golden vectors v001 to v021, run by the core on every build
- verifiedOn: 2026-09-23

The reference cannot be GeographicLib, and that is worth stating plainly: GeographicLib computes the ellipsoidal answer, which is the number this tool is deliberately *not* producing. Checking against it would fail for the reason the tool exists. So the formulas are transcribed from their own definitions in `tools/vectors/gen_nav_sphere_vincenty.py`, with the same mean radius, and the ellipsoidal figure the tool reports alongside is separately checked by `navigation.geodesic.inverse`.

## Differential tests

- `tools/vectors/gen_nav_sphere_vincenty.py`: 16 of the 21 vectors, from the formulas
- `core/crates/gp-navigation/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/navigation.geodesic.spherical-inverse.jsonl`: 21 pairs across the globe

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `spherical_inverse_invariants`: a quarter of the equator is exactly R·π/2, which is the one case the formula can be checked against arithmetic alone; the distance is symmetric, and the courses swap when the points do; due east along the equator gives a course of exactly 90° and due north along a meridian exactly 0°; the spherical distance differs from the ellipsoidal one by under half a percent everywhere tried, and the reported difference is exactly that gap; and a larger radius scales the distance in proportion, since the sphere enters only as a multiplier
