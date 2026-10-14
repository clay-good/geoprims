<!-- Derivation note (trust/correctness-program layer A). -->
# East, north, up to azimuth, elevation, and range (`geodesy.frame.enu-to-aer`)

## Method

Offsets along the local east, north, and up axes are Cartesian coordinates in the observer's tangent frame. Their length is the slant range, the direction of their horizontal part is the azimuth, and the angle they make with the horizontal is the elevation. The operation calls the same function `geodesy.frame.to-local` uses.

## Equations

- Range r = √(e² + n² + u²)
- Azimuth = atan2(e, n), wrapped to 0°–360°
- Elevation = atan2(u, √(e² + n²))

## Symbols and units

e, n, u offsets in meters or any length unit; azimuth clockwise from north and elevation above the horizontal, in degrees.

## Domain

Offsets to 100,000 km. A target with no horizontal offset has no azimuth: 0 is returned with AZIMUTH_UNDEFINED and the elevation is exactly ±90°.

## Approximations

A flat tangent frame: no Earth curvature or refraction.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-geodesy-suite specification
- sourceEdition: 2026
- sourceLocator: geodesy/local-frames, requirement "Local tangent frames"; checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: east 3 m, north 4 m, up 12 m
- outputs: horizontal 5 m, range 13 m, azimuth 36.8699°, elevation 67.3801°
- tolerance: 1e-7
- verifiedBy: golden vector v005 and the `focused_frame_and_grid_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `enu_to_aer_vectors`: 14 random offsets to 50 km and six axis cases, by trigonometry in Python.
- `core/vectors/geodesy.frame.enu-to-aer.jsonl`: those vectors and two refusal cases.
- The integration suite requires the same look angles as `geodesy.frame.to-local` for that tool's own offsets.

## Invariants

- The range is never less than the horizontal distance.
- Negating all three offsets turns the azimuth by 180° and negates the elevation.
- Converting the result with `geodesy.frame.aer-to-enu` returns the offsets.
