<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Look angles (`navigation.vector.look-angles`)

## Method

Where to point: the azimuth, elevation and straight-line range from an observer to a target, both with heights. An antenna, a camera, a tracking mount, a line-of-sight check.

Both points go to earth-centred coordinates and the difference vector is rotated into **east-north-up** at the observer. Then azimuth = atan2(E, N) and elevation = atan2(U, √(E² + N²)). Doing it this way rather than with a flat-earth triangle matters as soon as the range is more than a few kilometres: the target sinks below the tangent plane as the Earth curves away, and a flat calculation says it is above the horizon when it is not.

The tool reports the **horizon elevation** alongside — the depression angle of the true horizon from the observer's height, −acos(Rₑ/(Rₑ + h)) — so the comparison that actually matters, is the target above the horizon, is one subtraction rather than a judgement.

It also reports an **apparent elevation**, raised by atmospheric refraction, with a k-factor for the effective Earth radius. Refraction bends a ray downward, so a target slightly below the geometric horizon can still be visible; the standard k = 4/3 is what radio and survey practice assumes.

## Equations

- ENU as in `navigation.vector.distance-3d`.
- azimuth = atan2(E, N), wrapped to [0, 360); elevation = atan2(U, √(E² + N²)).
- slant range = √(E² + N² + U²).
- horizon elevation = −acos(Rₑ / (Rₑ + h)), with Rₑ the Earth's mean radius.
- Refraction: the effective radius is k·Rₑ, which raises the apparent elevation.

## Symbols and units

`observer_lat`, `observer_lon`, `observer_height` and the target's three, with `k` for the refraction factor. Out come `azimuth`, `elevation`, `slant_range`, `apparent_elevation` and `horizon_elevation`.

## Domain

Any observer and target. A target directly overhead has an elevation of 90° and no meaningful azimuth.

## Approximations

The geometric angles are exact. The refraction model is a standard effective-radius approximation, which is good for near-horizontal paths in a well-mixed atmosphere and poor in a temperature inversion — where ducting can carry a signal far past where any k-factor predicts.

## Worked example

- sourcePublisher: PROJ contributors
- sourceTitle: PROJ through pyproj, EPSG:4979 to EPSG:4978 (WGS 84 geodetic to geocentric)
- sourceEdition: PROJ 9.3.0 / pyproj 3.6.1
- sourceLocator: the geodetic-to-geocentric transformation, with the east-north-up rotation applied from its own definition
- independent: yes
- inputs: 21 observer-and-target pairs, including one due east, one due north, one below the horizon, a near-polar pair, and observers from sea level to 2,240 m
- outputs: the azimuth, elevation, slant range and horizon elevation
- tolerance: 1e-9° on the angles, 1e-6 m on the range
- verifiedBy: golden vectors v001 to v021, run by the core on every build
- verifiedOn: 2026-09-23

One case in the reference had to be corrected twice, and both corrections are worth recording. The horizon angle uses an Earth radius of 6,371,000 m, not the 6,371,008.8 m that is the usual quoted mean radius of WGS 84 — 8.8 m of difference, which moves the angle by a part in a million and is invisible at any loose tolerance and obvious at 1e-9°. And a target due north comes out as a vanishingly small negative azimuth whose remainder modulo 360 rounds up to exactly 360.0, which is the one value the range [0, 360) excludes.

## Differential tests

- `tools/vectors/gen_nav_vector.py`: 12 of the 17 vectors, ECEF from PROJ
- `core/crates/gp-navigation/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/navigation.vector.look-angles.jsonl`: 21 pairs from the equator to 89.5° north

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `look_angles_invariants`: the azimuth is in [0, 360) and the elevation in [−90, 90]; a target due east of the observer gives an azimuth of 90° and one due north gives 0°, not 360°; the slant range matches what `navigation.vector.distance-3d` gives for the same two points, so the two tools do not carry separate geometry; a target directly overhead has an elevation of 90°; the horizon elevation is negative, deepens with the observer's height, and is zero at sea level; a distant low target sits below the horizon and a near one above it; and the apparent elevation, which appears only when a refraction factor is asked for — without one the tool reports the geometry alone rather than assuming an atmosphere — is above the geometric one, since refraction only ever bends a ray toward the ground
