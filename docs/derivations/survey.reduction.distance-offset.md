<!-- Derivation note (trust/correctness-program layer A). -->
# Distance offset shot (`survey.reduction.distance-offset`)

## Method

A total-station shot is taken to a prism held beside the point that cannot be occupied, and the gap is taped: right or left of the line of sight, and beyond or short of the prism along it. The operation moves the prism position by those offsets in the line-of-sight frame. It shares one calculation path with `survey.cogo.offset-shot`.

## Equations

For station (N₀, E₀), azimuth α to the prism, horizontal distance d, offset out o, and offset right r:

- N = N₀ + (d + o)·cos α − r·sin α
- E = E₀ + (d + o)·sin α + r·cos α
- distance = hypot(N − N₀, E − E₀), azimuth = atan2(E − E₀, N − N₀)

## Symbols and units

Coordinates, the distance, and both offsets use one compatible length unit: meters, international feet, or US survey feet. α is a north azimuth, entered as an azimuth or a quadrant bearing. Right is positive as seen from the instrument; left is negative. Out is positive beyond the prism; in is negative.

## Domain

The distance must be positive and at least one offset must be given. Mixing international and US survey feet is refused with UNIT_MISMATCH.

## Approximations

Exact plane geometry subject to floating-point evaluation. The distance must already be horizontal. Offsets are assumed square to the line of sight or along it; a tape held at a skew is not modeled.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-survey-suite specification
- sourceEdition: 2026
- sourceLocator: survey/instrument-reductions offset-shot requirement; the geometry is checked by inspection, not taken from a published table
- independent: no
- inputs: station N 0.00 m, E 0.00 m; direction 0° (north); distance 100.00 m; offset right 3.00 m; offset out −2.00 m
- outputs: N 98.00 m, E 3.00 m
- tolerance: 1e-9 m
- verifiedBy: the `named_offset_shots_match_the_general_tool` integration test, by inspection of the geometry (looking north, right is east)
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` `named_offset`: an independent Python rotation of the offsets into grid axes at 19 random cases across all three length units, alternating decimal azimuths with quadrant bearings and right-only, out-only, and combined offsets.
- `core/vectors/survey.reduction.distance-offset.jsonl`: 20 successful shots and four refusal cases.
- The integration suite requires exact result, display, and warning parity with `survey.cogo.offset-shot`.

## Invariants

- With both offsets zero the point is the prism.
- An out offset changes only the distance along the line of sight.
- Reversing the sign of the right offset mirrors the point across the line of sight.
- The reported distance and azimuth are the inverse from the station to the reported point.
