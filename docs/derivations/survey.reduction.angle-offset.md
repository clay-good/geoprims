<!-- Derivation note (trust/correctness-program layer A). -->
# Angle offset shot (`survey.reduction.angle-offset`)

## Method

For a round object such as a tree or pole, the prism is held beside it at the distance of its near face and the distance is measured. The instrument is then turned to the center of the object and the direction is read. The center lies on that direction, one radius beyond the measured distance. The operation shares one calculation path with `survey.cogo.offset-shot`.

## Equations

For station (N₀, E₀), measured distance d, radius r, and azimuth α꜀ turned to the center:

- N = N₀ + (d + r)·cos α꜀
- E = E₀ + (d + r)·sin α꜀
- distance = d + r

## Symbols and units

Coordinates, the distance, and the radius use one compatible length unit: meters, international feet, or US survey feet. Directions are north azimuths, entered as azimuths or quadrant bearings. The direction to the prism is recorded with the shot; only the direction to the center positions the result.

## Domain

The distance must be positive and the radius zero or more. Both the direction to the center and the radius are required. Mixing international and US survey feet is refused with UNIT_MISMATCH.

## Approximations

Exact plane geometry subject to floating-point evaluation. The object is assumed round and the prism is assumed to be at the same distance as the object's near face. The distance must already be horizontal.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-survey-suite specification
- sourceEdition: 2026
- sourceLocator: survey/instrument-reductions tree-center scenario; the geometry is checked by inspection, not taken from a published table
- independent: no
- inputs: station N 0.00 m, E 0.00 m; direction to the prism 88°; distance 40.00 m; direction to the center 90° (east); radius 0.50 m
- outputs: N 0.00 m, E 40.50 m
- tolerance: 1e-9 m
- verifiedBy: the `named_offset_shots_match_the_general_tool` integration test, by inspection of the geometry (the center is due east at distance plus radius)
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` `named_offset`: an independent Python evaluation at 19 random cases across all three length units, with centers turned up to 2° either side of the prism and radii from 0.05 to 3 units.
- `core/vectors/survey.reduction.angle-offset.jsonl`: 20 successful shots, including the tree-center scenario, and four refusal cases.
- The integration suite requires exact result, display, and warning parity with `survey.cogo.offset-shot`.

## Invariants

- The reported distance equals the measured distance plus the radius.
- The reported azimuth equals the direction turned to the center.
- A zero radius places the point at the measured distance on the center direction.
