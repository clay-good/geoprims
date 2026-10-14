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

- sourcePublisher: geoprims
- sourceTitle: add-geodesy-suite specification
- sourceEdition: 2026
- sourceLocator: geodesy/local-frames, requirement "Local tangent frames" (rotation matrix); checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: latitude 0°, longitude 0°, ENU
- outputs: rows (0, 1, 0), (0, 0, 1), (1, 0, 0): east is +Y, north is +Z, up is +X
- tolerance: exact
- verifiedBy: golden vector v002 and the `focused_frame_and_grid_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `rotation_vectors`: 14 random places and six axis cases in both frames, by the closed-form rows in Python, to 1e-14.
- `core/vectors/geodesy.frame.local-rotation-matrix.jsonl`: those vectors and one refusal case.
- The integration suite requires the matrix `geodesy.frame.to-local` reports and checks orthonormality.

## Invariants

- The rows are unit vectors at right angles to one another.
- The NED matrix is the ENU matrix with its first two rows swapped and its third negated.
- The transpose is the inverse.
