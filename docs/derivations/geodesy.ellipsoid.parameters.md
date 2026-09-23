<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Ellipsoid parameters (`geodesy.ellipsoid.parameters`)

## Method

An ellipsoid of revolution is fixed by two numbers. Every other figure anyone quotes for it — the semi-minor axis, the eccentricities, the third flattening, and the radii of the spheres that match it in one respect each — follows from those two by exact algebra, so the tool takes the defining pair, by name or given directly, and writes the rest out.

## Equations

- b = a(1 − f), and f = 1 − b/a when the pair given is a and b.
- First eccentricity squared: e² = f(2 − f); second: e′² = e²/(1 − e²).
- Third flattening: n = f/(2 − f).
- Mean radius R₁ = (2a + b)/3.
- Authalic radius R₂, the sphere of equal area: R₂ = √(A/4π) with A the ellipsoid's surface area, A = 2πa²(1 + (1 − e²)/(2e)·ln((1 + e)/(1 − e))).
- Volumetric radius R₃ = ∛(a²b).

## Symbols and units

a is the semi-major axis in metres, f the flattening, usually quoted as its reciprocal 1/f. b, and the three radii, are in metres; e², e′², f, and n are dimensionless. A named ellipsoid is looked up in the registry; a custom one takes a with either f, 1/f, or b.

## Domain

Any oblate ellipsoid: 0 ≤ f < 1, with a > 0. A sphere, f = 0, is allowed and gives e² = 0 and three equal radii. Prolate figures, b > a, are refused, since the formulae here are written for the oblate case the Earth is.

## Approximations

None. The relations are exact identities, not fits, and the only error is the floating point they are evaluated in. Against a 40-digit evaluation of the same relations, the values agree to the last bit a double carries.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: Department of Defense World Geodetic System 1984
- sourceEdition: NGA.STND.0036_1.0.0_WGS84 (the edition the sources ledger records, checked at the issuer on 2026-09-19)
- sourceLocator: the WGS 84 defining parameters, semi-major axis a = 6378137 m and reciprocal flattening 1/f = 298.257223563, with the derived semi-minor axis b = 6356752.3142 m and first eccentricity squared e² = 0.00669437999014
- independent: yes
- inputs: ellipsoid wgs84
- outputs: b 6356752.314245179 m, f 0.003352810664747481, e² 0.006694379990141317, mean radius 6371008.771415059 m
- tolerance: the published b to 0.0001 m and e² to 1e-14, which is the precision the standard prints them to
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-22

## Differential tests

- `tools/vectors/gen_frames.py`: mpmath at 40 digits, evaluating the defining relations independently of the core, over every named ellipsoid in the registry
- `core/vectors/geodesy.ellipsoid.parameters.jsonl`: 23 vectors from that evaluation, run through the core on every build

## Invariants

- `core/crates/gp-geodesy/tests/frames.rs` `ellipsoid_parameter_invariants`: over five ellipsoids, each derived value is checked against the two that define the figure — b = a(1 − f), e² = f(2 − f), e′² = e²/(1 − e²), n = f/(2 − f), R₁ = (2a + b)/3 — and the authalic and volumetric radii lie between the axes and below the mean radius
- `core/crates/gp-geodesy/tests/frames.rs` `wgs84_parameters`: the WGS 84 values against the published constants
