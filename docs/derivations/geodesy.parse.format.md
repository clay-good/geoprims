<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Coordinate formatting (`geodesy.parse.format`)

## Method

Writes a latitude and longitude the three ways they are written — decimal degrees, degrees and decimal minutes, degrees minutes seconds — at a precision you choose, and says what that precision is worth on the ground.

The formatting has the same carry problem the angle arithmetic does, and the same answer: round once, at the printed unit, then split into components. Rounding a seconds field on its own gives 59'60", and rounding a minutes field on its own gives 59.9999' when the intent was a whole degree. Here 10.9999999° to whole seconds is 11°00'00", not 10°59'60".

The second output is the one that changes how people use the tool. A coordinate's precision is usually quoted in digits, which tells you nothing without a latitude: six decimal places of longitude is 11 cm at the equator and 5 cm at 60° north, and three decimal places of a minute is a different distance again. So the last printed digit is converted to metres, separately for latitude and longitude, using the WGS 84 radii of curvature at that latitude — the meridian radius M for north-south and the parallel radius N cos φ for east-west.

## Equations

- Step of the last digit: 10^−d degrees for `dd`, 10^−d/60 for `ddm`, 10^−d/3600 for `dms`.
- Meridian radius: M(φ) = a(1 − e²) / (1 − e² sin²φ)^{3/2}.
- Prime vertical radius: N(φ) = a / √(1 − e² sin²φ).
- Latitude resolution = step · M(φ) · π/180; longitude resolution = step · N(φ) cos φ · π/180.
- WGS 84: a = 6,378,137 m, 1/f = 298.257223563.

## Symbols and units

`lat` and `lon` in degrees; `style` is `dd`, `ddm` or `dms`; `decimals` is how many places on the last component; `signs` chooses hemisphere letters or signed numbers. Out come `formatted` and the two resolutions in metres.

## Domain

Any latitude and longitude. At the poles the longitude resolution goes to zero, which is correct: a degree of longitude is no distance there.

## Approximations

None in the formatting. The resolutions are exact for the WGS 84 ellipsoid at that latitude, and they describe the last digit's step, not the accuracy of the number itself — a coordinate printed to six places is not necessarily good to 11 cm.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: WGS 84 (NGA.STND.0036), and the standard radii of curvature
- sourceEdition: WGS 84: a = 6,378,137 m, 1/f = 298.257223563
- sourceLocator: meridian radius M and prime vertical radius N as functions of latitude
- independent: yes
- inputs: 21 coordinates in all three styles with 0 to 8 decimals, including the origin, a value that carries all the way to a whole degree, points a ten-millionth of a degree from 90° and 180°, and both hemispheres
- outputs: the formatted string and both ground resolutions
- tolerance: exact on the string, 1e-9 relative on the resolutions
- verifiedBy: golden vectors v001 to v021, run by the core on every build
- verifiedOn: 2026-09-23

The resolutions are recomputed in `tools/vectors/gen_parse_more.py` from the two radius formulas and the WGS 84 constants, not read back from the core. The cases at 89.9999999° and −89.9999999° are there because that is where the two resolutions diverge most: a ten-thousandth of an arcsecond of latitude is about 3 mm anywhere, while the same step of longitude is about 5 µm up there.

## Differential tests

- `tools/vectors/gen_parse_more.py`: 16 of the 21 vectors, from the WGS 84 radii of curvature
- `core/crates/gp-geodesy/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/geodesy.parse.format.jsonl`: 21 coordinates across all three styles

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `format_invariants`: the carry reaches the degrees, so 10.9999999° to whole seconds is 11°00'00" and never 10°59'60"; every formatted coordinate parses back through `geodesy.parse.coordinates` to within the resolution it claims, in all three styles, which ties the string to the number it came from; a longitude resolution shrinks with latitude while the latitude resolution barely moves, since one follows cos φ and the other does not; one more decimal place divides both resolutions by ten in `dd`, and the step between styles is the 60 and 3600 the units say; and at the equator six decimal places of longitude comes to about 11 cm, the figure this is usually quoted as
