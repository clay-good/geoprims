<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Haversine distance (`navigation.geodesic.haversine`)

## Method

The great-circle distance on a sphere by the haversine formula, which stays well-conditioned for short distances. It runs beside Karney's geodesic on WGS 84 and reports the difference, so it is clear how far the sphere is from the Earth.

## Equations

- h = sin²(Δφ/2) + cos φ1 cos φ2 sin²(Δλ/2).
- d = 2R asin(√h), with h clamped to at most 1.
- Difference = Karney s12 on WGS 84 − d.

## Symbols and units

φ latitudes and λ longitudes (radians in the formula, degrees at the interface), R the sphere radius (default R1 = 6,371,008.771 m, the IUGG mean radius), d distance (km by default).

## Domain

Any two points. Antipodal points are fine for the haversine. The Karney comparison also handles them.

## Approximations

Exact on the sphere. On the Earth, the sphere is off by up to about 0.5%, and the result reports that difference against the ellipsoidal geodesic. Near-antipodal points lose some precision in asin(√h), a few millimeters at most.

## Worked example

- sourcePublisher: Rosetta Code
- sourceTitle: Haversine formula (task description)
- sourceEdition: retrieved 2026-09-19
- sourceLocator: Nashville BNA (36.12, −86.67) to Los Angeles LAX (33.94, −118.40) on a 6,372.8 km sphere is about 2,887.26 km
- independent: yes
- inputs: 36.12, −86.67 to 33.94, −118.40, radius 6,372.8 km
- outputs: 2,887.26 km (the core gives 2,887.2599506 km)
- tolerance: 0.005 km (the printed rounding)
- verifiedBy: golden vector v020, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-navigation/tests/navigation.rs`: `haversine_matches_independent_implementations` runs 1,000 short, regional, and global lines against a separate Python haversine (within 1 µm) and GeographicLib 2.1's ellipsoidal distance (within 1 µm)
- `tools/vectors/gen_dev_diff.py`: regenerates that fixture
- `core/vectors/navigation.geodesic.haversine.jsonl`: 20 vectors, including antipodes, poles, the antimeridian, and a zero-length line

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `haversine_invariants`: symmetric, zero for a point to itself, obeys the triangle inequality, at most half the circumference, and linear in the radius
