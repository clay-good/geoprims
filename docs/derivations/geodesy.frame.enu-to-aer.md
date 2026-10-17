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

- sourcePublisher: The MathWorks
- sourceTitle: Mapping Toolbox documentation, enu2aer
- sourceEdition: retrieved 2026-10-17
- sourceLocator: worked example, a ground vehicle at east 8.4504 m, north 12.4737 m, up 1.1046 m
- independent: yes
- inputs: east 8.4504 m, north 12.4737 m, up 1.1046 m
- outputs: azimuth 34.1160°, elevation 4.1931°, range 15.1070 m
- tolerance: the documented four decimals (5e-5° on the angles, 5e-5 m on the range)
- verifiedBy: the `focused_frame_and_grid_operations_match_their_general_tools` integration test, which requires parity with the stable `geodesy.frame.to-local` and a clean ENU round trip
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `enu_to_aer_vectors`: 14 random offsets to 50 km and six axis cases, by trigonometry in Python.
- `core/vectors/geodesy.frame.enu-to-aer.jsonl`: those vectors and two refusal cases.
- The integration suite requires the same look angles as `geodesy.frame.to-local` for that tool's own offsets.

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `focused_frame_and_grid_operations_match_their_general_tools`: the look angles match the stable `geodesy.frame.to-local` on the same offsets, a round trip through `aer-to-enu` returns them, and a straight-up offset reports a 90° elevation and AZIMUTH_UNDEFINED.
- The range is never less than the horizontal distance.
- Negating all three offsets turns the azimuth by 180° and negates the elevation.
