<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Latitude and longitude to an Irish Grid reference (`geodesy.grid-ref.irish-forward`)

## Method

Irish and Northern Irish maps, walking guides, and mountain rescue reports give places as Irish Grid references: one letter for a 100 km square, then figures. The grid is EPSG 29903, TM75 / Irish Grid: transverse Mercator on the Airy Modified 1849 ellipsoid, true origin 53°30′N 8°W, scale 1.000035, false origin 200 km west and 250 km south of it. The tool takes three steps:

1. **Onto TM75.** A WGS 84 latitude and longitude is carried to TM75 by EPSG operation 1954, a seven-parameter position-vector transformation stated at 1 m. A TM75 latitude and longitude skips this step.
2. **Onto the grid.** Krüger's series in Karney's arrangement, with the grid's parameters.
3. **Lettered.** The 500 km square is cut into 25 squares of 100 km, lettered A to Z without I in rows of five from the north-west, so V's south-west corner is the false origin and A is the north-west square. The figures follow, half for the easting and half for the northing, **truncated**, never rounded, so a reference names the square the point is in.

| Letters | Northing |
|---|---|
| A B C D E | 400 to 500 km |
| F G H J K | 300 to 400 km |
| L M N O P | 200 to 300 km |
| Q R S T U | 100 to 200 km |
| V W X Y Z | 0 to 100 km |

## Equations

- Letter: `col = ⌊E/100 km⌋`, `row = 4 − ⌊N/100 km⌋`, letter = `ABCDEFGHJKLMNOPQRSTUVWXYZ[5·row + col]`.
- Figures with `d` digits each: `⌊(E mod 100 km)/10^(5−d) m⌋`, and the same for N.
- The square's side: `10^(5−d)` m.
- Helmert (position vector): `X′ = T + (1 + s)·R·X`, with `T = (482.5, −130.6, 564.6)` m, `R` the small rotations `(−1.042″, −0.214″, −0.631″)`, `s = 8.15` ppm.

## Symbols and units

Latitude and longitude in degrees on WGS 84 or TM75; easting and northing in meters from the Irish Grid's false origin; the reference as text; the square's side and bounds in meters and degrees.

## Domain

- 2, 4, 6, 8, or 10 figures; an odd count is refused.
- Outside the lettered 500 km square there is no letter, and the tool refuses with `OUT_OF_DOMAIN`.
- Outside EPSG's area for the grid and its datum, Ireland and Northern Ireland onshore, the answer carries `OUTSIDE_AREA_OF_USE`.

## Approximations

From WGS 84 the transformation is EPSG's seven-parameter fit, about 1 m: the `LOW_ACCURACY_TRANSFORM` warning. The OSi and OSNI polynomial transformation, good to 0.4 m, is a separate step of the specification and not applied here, so a ten-figure reference is more precise than it is accurate.

## Worked example

- sourcePublisher: IOGP, through PROJ
- sourceTitle: TM75 / Irish Grid (EPSG 29903) and TM75 to WGS 84 (2) (EPSG 1954)
- sourceEdition: EPSG v13.102, as shipped in PROJ 9.9.0
- sourceLocator: Both operations applied by PROJ to the Spire of Dublin at 53.349804°N 6.260310°W on WGS 84
- independent: yes
- inputs: lat 53.349804, lon -6.26031, figures 10
- outputs: O 15899 34671; E 315899.885 m, N 234671.788 m
- tolerance: 0.001 m on the easting and northing; the reference exactly
- verifiedBy: golden vector v001 and `core/crates/gp-geo/src/gridref.rs` `letters_squares_and_round_trips`
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_irish.py` builds all 40 vectors in `core/vectors/geodesy.grid-ref.irish-forward.jsonl` from PROJ 9.9.0 through `cs2cs`, with EPSG 1954 written out as `+towgs84` so PROJ cannot choose another operation (both definitions are checked against `projinfo` on every run): ten places from Mizen Head to Malin Head and Achill Head to Slieve Donard, from WGS 84 and from TM75, at 10 and 6 figures, within a millimeter. The references come from a second lettering written in the script itself, not from the core.

## Invariants

- `core/crates/gp-geo/src/gridref.rs` `letters_squares_and_round_trips`: the false origin is in V and the north-west square is A; every one of the 25 squares at every precision encodes to a reference whose decoded square holds the point and encodes back to the same reference; I, odd figure counts, and more than ten figures are refused.
- `core/crates/gp-search/src/detect.rs` `british_and_irish_grid_references`: a pasted Irish Grid reference is recognized and sent to the inverse tool, and an altimeter group of the same shape is not.
