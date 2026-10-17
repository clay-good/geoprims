<!-- Derivation note (trust/correctness-program layer A). -->
# Magnetic bearing to true bearing (`geodesy.magnetic.magnetic-to-true`)

## Method

A compass points along the magnetic meridian, which differs from true north by the variation. Adding easterly variation to a magnetic bearing, or subtracting westerly, gives the true bearing. The operation is `geodesy.magnetic.true-to-magnetic` with its direction fixed.

## Equations

- True = magnetic + variation, with east positive and west negative
- The result is wrapped to 0° to 360°

## Symbols and units

Bearings in degrees clockwise from north. Variation as printed on a chart (12°W, 3.5 E) or signed, or taken from the magnetic model for a latitude, longitude, and date.

## Domain

A chart variation, or a place and date within the model's years.

## Approximations

Deviation of the compass in its vehicle is not included.

## Worked example

- sourcePublisher: NOAA National Centers for Environmental Information
- sourceTitle: WMM2025 test values (WMM2025_TestValues.txt)
- sourceEdition: WMM2025, released 2024-11-13
- sourceLocator: Row 2 (2025.0, 48 km, 80° N, 96° W), D = −29.91°; applied to a 100° magnetic bearing
- independent: yes
- inputs: 100° magnetic at 80° N, 96° W, 48 km, 2025.0
- outputs: variation 29.91° W (−29.91°), 70.09° true
- tolerance: 0.005° on the variation (the test file prints D to 0.01°)
- verifiedBy: the `magnetic_to_true_invariants` test in `core/crates/gp-geodesy/tests/geodesy_views_invariants.rs`, which requires the variation to equal the WMM2025 declination tool's at this point
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `magnetic_vectors`: 20 bearings with chart variations in five written forms, including results that cross north.
- `core/vectors/geodesy.magnetic.magnetic-to-true.jsonl`: those vectors and two refusal cases.
- The integration suite requires the whole result to equal `geodesy.magnetic.true-to-magnetic` in its magnetic-to-true direction.

## Invariants

- `core/crates/gp-geodesy/tests/geodesy_views_invariants.rs` `magnetic_to_true_invariants`: in model mode the variation applied equals the WMM2025 declination tool's at the same point, the true bearing is the magnetic bearing plus the east-positive variation wrapped to [0°, 360°), `true-to-magnetic` reads it back, and zero variation leaves the bearing unchanged.
