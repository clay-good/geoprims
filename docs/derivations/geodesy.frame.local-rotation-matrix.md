<!-- Derivation note (trust/correctness-program layer A). -->
# ECEF to local frame rotation matrix (`geodesy.frame.local-rotation-matrix`)

## Method

The local east, north, and up directions at a place are three unit vectors in Earth-centered, Earth-fixed axes. Written as the rows of a matrix, they rotate any ECEF vector into local components. The operation returns that matrix, or its north-east-down form, from the same function `geodesy.frame.to-local` uses.

## Equations

At geodetic latitude φ and longitude λ:

- east = (−sin λ, cos λ, 0)
- north = (−sin φ cos λ, −sin φ sin λ, cos φ)
- up = (cos φ cos λ, cos φ sin λ, sin φ)
- ENU matrix rows: east, north, up. NED matrix rows: north, east, −up.

## Symbols and units

φ, λ in degrees. Matrix entries are direction cosines, unitless, between −1 and 1.

## Domain

Latitude from −90° to 90°.

## Approximations

None. The up row is the ellipsoid normal at the geodetic latitude, not the geocentric direction.

## Worked example

- sourcePublisher: MathWorks
- sourceTitle: Mapping Toolbox documentation, ecef2enu and ecef2ned
- sourceEdition: R2024b online documentation
- sourceLocator: mathworks.com/help/map/ref/ecef2enu.html and mathworks.com/help/map/ref/ecef2ned.html, the numeric examples
- independent: yes
- inputs: ENU at latitude 45.9132°, longitude 36.7484°, applied to the ECEF offset of the ecef2enu example (point 5,507.5289, 4,556.2241, 6,012.8208 km; origin at height 1,877.7532 km on WGS 84); NED at latitude 44.532°, longitude −72.782°, applied to the ecef2ned example (point 1,345.660, −4,350.891, 4,452.314 km; origin height 1.699 km)
- outputs: east 355.6013, north −923.0832, up 1,041.0 km; and north 1.3343, east −2.5444, down 0.3600 km, matching every digit the pages print
- tolerance: the pages' printed precision (1e-4 km) for the products; 1e-12 for the matrix entries against the closed-form rows in Python
- verifiedBy: golden vectors v022 and v023; the ECEF origin was computed separately from the WGS 84 closed form
- verifiedOn: 2026-10-19

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `rotation_vectors`: 14 random places and six axis cases in both frames, by the closed-form rows in Python, to 1e-14.
- `core/vectors/geodesy.frame.local-rotation-matrix.jsonl`: those vectors, one refusal case, and the two MathWorks origins (v022, v023).
- The integration suite requires the matrix `geodesy.frame.to-local` reports and checks orthonormality.

## Invariants

- `core/crates/gp-geodesy/tests/geodesy_views_invariants.rs` `local_frame_views_invariants`: at six places, including near a pole and on the antimeridian, the rows are orthonormal, east × north = up, up is the ellipsoid normal, and the NED matrix is the ENU matrix with its first two rows swapped and its third negated (so the transpose is the inverse).
