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

- sourcePublisher: geoprims
- sourceTitle: add-geodesy-suite specification
- sourceEdition: 2026
- sourceLocator: geodesy/local-frames, requirement "Local tangent frames"; checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: azimuth 0°, elevation 0°, range 1,000 m
- outputs: east 0, north 1,000 m, up 0
- tolerance: 1e-9 m
- verifiedBy: golden vector v002 and the `focused_frame_and_grid_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `aer_to_enu_vectors`: 14 random directions and ranges to 400 km and six axis cases, by trigonometry in Python.
- `core/vectors/geodesy.frame.aer-to-enu.jsonl`: those vectors and two refusal cases.
- The integration suite converts the look angles of `geodesy.frame.to-local` back to its offsets.

## Invariants

- east² + north² + up² = range².
- Converting the result with `geodesy.frame.enu-to-aer` returns the inputs.
- An elevation of 90° puts all of the range in up.
