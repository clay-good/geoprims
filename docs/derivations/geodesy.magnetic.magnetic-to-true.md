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

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge
- sourceEdition: FAA-H-8083-25C (2023)
- sourceLocator: Chapter 16, variation: "east is least, west is best"; the numbers below are this note's arithmetic on that rule, not a printed example
- independent: no
- inputs: 215° magnetic, variation 9°E
- outputs: 224° true
- tolerance: 1e-9°
- verifiedBy: golden vector v001 and the `focused_parse_format_and_ellipsoid_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `magnetic_vectors`: 20 bearings with chart variations in five written forms, including results that cross north.
- `core/vectors/geodesy.magnetic.magnetic-to-true.jsonl`: those vectors and two refusal cases.
- The integration suite requires the whole result to equal `geodesy.magnetic.true-to-magnetic` in its magnetic-to-true direction.

## Invariants

- Converting the result back with `geodesy.magnetic.true-to-magnetic` returns the bearing.
- Zero variation leaves the bearing unchanged.
- The result is always from 0° up to 360°.
