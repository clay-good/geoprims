<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Straight-line distance in three dimensions (`navigation.vector.distance-3d`)

## Method

The distance between two points that are not at the same height — a drone and its ground station, an aircraft and a transmitter, a summit and a valley floor. That is not the distance along the ground, and above a few kilometres it is not the ground distance with Pythagoras applied to the height difference either, because the ground curves away underneath.

The method is to leave the surface entirely. Both points go to **earth-centred, earth-fixed** coordinates — a single Cartesian frame with its origin at the centre of the ellipsoid — and the straight-line distance is then the plain length of the difference vector. No curvature correction is needed because nothing is on a curved surface any more.

Heights have to be the same kind first. A GPS height is above the ellipsoid and a map height is above sea level, and they differ by tens of metres; give the tool which kind each is and it converts through the geoid before doing anything else.

The elevation angle and azimuth come from rotating that difference vector into east-north-up at the first point. For comparison the tool also reports the flat-earth answer — √(ground² + Δh²) — so the error of the approximation is visible rather than assumed small.

## Equations

- Geodetic to ECEF: X = (N + h) cos φ cos λ, Y = (N + h) cos φ sin λ, Z = (N(1 − e²) + h) sin φ, with N = a/√(1 − e² sin²φ).
- Slant range = |ΔECEF|.
- ENU rotation at (φ, λ): E = −sin λ ΔX + cos λ ΔY; N = −sin φ cos λ ΔX − sin φ sin λ ΔY + cos φ ΔZ; U = cos φ cos λ ΔX + cos φ sin λ ΔY + sin φ ΔZ.
- Elevation = atan2(U, √(E² + N²)); azimuth = atan2(E, N).

## Symbols and units

`lat1`, `lon1`, `height1` and the same for the second point, with `reference1` and `reference2` saying whether each height is `hae` (above the ellipsoid) or `msl` (above mean sea level). Out come the `slant_range`, `ground_distance`, `height_difference`, `elevation_angle`, `azimuth`, and the flat-earth pair.

## Domain

Any two points, any heights. The geoid conversion needs the EGM96 asset when a height is given as orthometric.

## Approximations

None in the slant range: ECEF is exact and the difference of two exact positions is exact. The geoid model carries its own half-metre to metre, and only when an orthometric height is converted.

## Worked example

- sourcePublisher: PROJ contributors
- sourceTitle: PROJ through pyproj, EPSG:4979 to EPSG:4978 (WGS 84 geodetic to geocentric)
- sourceEdition: PROJ 9.3.0 / pyproj 3.6.1
- sourceLocator: the geodetic-to-geocentric transformation on WGS 84
- independent: yes
- inputs: 20 pairs, including two points on the same meridian, one directly above another, two on the equator a degree apart, a pair across the pole, and city pairs on five continents
- outputs: the slant range, the height difference and the elevation angle
- tolerance: 1e-6 m on the range, 1e-9° on the angle
- verifiedBy: golden vectors v001 to v020, run by the core on every build
- verifiedOn: 2026-09-23

PROJ does the geodetic-to-geocentric transform; the generator then does the subtraction and the east-north-up rotation from the matrix above. Neither step consults the core, and the rotation is written out rather than taken from a library, so a sign error in it would show as a wrong azimuth rather than as agreement by construction.

## Differential tests

- `tools/vectors/gen_nav_vector.py`: 12 of the 20 vectors, ECEF from PROJ
- `core/crates/gp-navigation/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/navigation.vector.distance-3d.jsonl`: 20 pairs across the globe

## Invariants

- `core/crates/gp-navigation/tests/navigation.rs` `distance_3d_invariants`: the slant range is symmetric, so swapping the points does not change it; it is never shorter than the height difference and never longer than ground distance plus height difference; for two points at the same height the slant range is slightly *shorter* than the ground distance, because a chord cuts under an arc — the sign of that difference is the check that the tool has left the surface; a point directly above another has a slant range equal to the height difference and an elevation of exactly 90°; the elevation angle is negative when looking down; and the flat-earth answer agrees closely over a kilometre and parts over a hundred, which is why both are reported
