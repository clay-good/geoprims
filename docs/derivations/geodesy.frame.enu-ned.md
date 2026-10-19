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

- sourcePublisher: MathWorks
- sourceTitle: Mapping Toolbox documentation, ecef2ned and ecef2enu
- sourceEdition: R2024b online documentation
- sourceLocator: mathworks.com/help/map/ref/ecef2ned.html and mathworks.com/help/map/ref/ecef2enu.html, the numeric examples
- independent: yes
- inputs: the ecef2ned example's printed result, NED north 1.3343 km, east −2.5444 km, down 0.3600 km; and the ecef2enu example's printed result, ENU east 355.6013 km, north −923.0832 km, up 1,041.0 km
- outputs: north 1,334.3 m, east −2,544.4 m, up −360 m; and north −923,083.2 m, east 355,601.3 m, down −1,041,000 m. The ENU form of the first agrees with the ENU rotation of the same ECEF offset (see `geodesy.frame.local-rotation-matrix`)
- tolerance: 1e-9 m and 1e-6 m in the golden vectors
- verifiedBy: golden vectors v024 and v025
- verifiedOn: 2026-10-19

## Differential tests

- `tools/vectors/gen_geodesy_focused.py` `enu_ned_vectors`: 19 random offsets entered alternately in each frame.
- `core/vectors/geodesy.frame.enu-ned.jsonl`: those vectors, the page's example, three refusal cases, and the two MathWorks results (v024, v025).
- The integration suite requires the same result whichever frame the offsets are entered in.

## Invariants

- `core/crates/gp-geodesy/tests/geodesy_views_invariants.rs` `local_frame_views_invariants`: the same offsets entered in either frame give the same four values, north and east pass through unchanged, and up + down = 0.
