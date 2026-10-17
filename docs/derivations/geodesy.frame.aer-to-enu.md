<!-- Derivation note (trust/correctness-program layer A). -->
# Azimuth, elevation, and range to east, north, up (`geodesy.frame.aer-to-enu`)

## Method

A direction and a distance from an observer are spherical coordinates in the observer's local tangent frame. Resolving them along the frame's axes gives the east, north, and up offsets. The operation calls the same function `geodesy.frame.from-local` uses for its azimuth-elevation-range input.

## Equations

- East = r·cos(el)·sin(az)
- North = r·cos(el)·cos(az)
- Up = r·sin(el); down = −up
- Horizontal distance = r·cos(el)

## Symbols and units

az azimuth clockwise from north; el elevation above the horizontal, both in degrees; r slant range. Offsets in meters.

## Domain

Elevation from −90° to 90°; range from 0 to 100,000 km.

## Approximations

A flat tangent frame: no Earth curvature or refraction.

## Worked example

- sourcePublisher: The MathWorks
- sourceTitle: Mapping Toolbox documentation, aer2enu
- sourceEdition: retrieved 2026-10-17
- sourceLocator: worked example, a sensor's line of sight at azimuth 34.1160°, elevation 4.1931°, slant range 15.1070 m
- independent: yes
- inputs: azimuth 34.1160°, elevation 4.1931°, range 15.1070 m
- outputs: east 8.4504 m, north 12.4737 m, up 1.1046 m
- tolerance: the documented four decimals (5e-5 m)
- verifiedBy: the `focused_frame_and_grid_operations_match_their_general_tools` integration test, which requires parity with the stable `geodesy.frame.to-local` on these offsets and a clean AER round trip
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `aer_to_enu_vectors`: 14 random directions and ranges to 400 km and six axis cases, by trigonometry in Python.
- `core/vectors/geodesy.frame.aer-to-enu.jsonl`: those vectors and two refusal cases.
- The integration suite converts the look angles of `geodesy.frame.to-local` back to its offsets.

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `focused_frame_and_grid_operations_match_their_general_tools`: the offsets match the stable `geodesy.frame.to-local` on the same line of sight, a round trip through `enu-to-aer` returns the inputs, and due north on the level puts all of the range in north.
- east² + north² + up² = range².
- An elevation of 90° puts all of the range in up.
