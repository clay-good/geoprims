<!-- Derivation note (trust/correctness-program layer A). -->
# ENU and NED frame converter (`geodesy.frame.enu-ned`)

## Method

East-north-up and north-east-down are two labelings of one local frame: the same origin, the same tangent plane, the same north and east. NED lists north first and measures the vertical downward so that its axes stay right-handed. The operation reorders the two horizontal offsets and negates the vertical one.

## Equations

- north_NED = north_ENU, east_NED = east_ENU
- down = −up

## Symbols and units

Offsets in meters or any length unit. The input frame is ENU unless NED is chosen.

## Domain

Offsets to 100,000 km. The vertical value of the chosen frame is required; the other, if given, must be its negative.

## Approximations

None: the conversion is exact.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-geodesy-suite specification
- sourceEdition: 2026
- sourceLocator: geodesy/local-frames, requirement "Local tangent frames"; checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: NED: north 300 m, east −120 m, down −45 m
- outputs: ENU: east −120 m, north 300 m, up 45 m
- tolerance: exact
- verifiedBy: golden vector v001 and the `focused_frame_and_grid_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `enu_ned_vectors`: 19 random offsets entered alternately in each frame.
- `core/vectors/geodesy.frame.enu-ned.jsonl`: those vectors, the page's example, and three refusal cases.
- The integration suite requires the same result whichever frame the offsets are entered in.

## Invariants

- Entering the same offsets in either frame gives the same result.
- up + down = 0.
- North and east are never changed.
