<!-- Derivation note (trust/correctness-program layer A). -->
# Radial sideshots from one station (`survey.cogo.radial-sideshots`)

## Method

Every sideshot begins at the same occupied station. The tool parses each entered quadrant bearing, north azimuth, or gon value into a north azimuth. It then calls the same plane forward-coordinate function used by `survey.cogo.forward`. Only the row handling is new: point names, directions, distances, and calculated coordinates stay together in one result table.

The station coordinates and every distance are converted to one common linear unit before calculation. International feet and US survey feet cannot be mixed because their 2 parts-per-million difference matters across survey distances. Each row is checked independently, and a bad direction or distance reports that row's exact input path.

## Equations

For station coordinates (Ns, Es), horizontal distance d, and north azimuth α:

- point northing N = Ns + d cos α
- point easting E = Es + d sin α

The equations are evaluated independently for every row. No row starts from the point calculated by the preceding row.

## Symbols and units

N and E are plane northing and easting coordinates. Ns and Es are the occupied-station coordinates. The horizontal distance d uses the same linear unit as the coordinates. The north azimuth α is measured clockwise from north in degrees after parsing, even when the input was a quadrant bearing, DMS angle, or gon value.

## Domain

At least one and at most 1,000 sideshots may be entered. Every row needs a point name, a valid direction, and a positive horizontal distance. All linear values must be compatible, and US survey feet cannot be mixed with international feet. Every direction must use the same grid or assumed north reference even though the text notation may differ by row.

## Approximations

The calculation is exact plane trigonometry apart from binary floating-point representation. It does not reduce a slope distance, apply an elevation or grid scale factor, rotate between north references, or account for Earth curvature. Those reductions must be completed before the coordinates are calculated.

## Worked example

- sourcePublisher: GeoPrims project
- sourceTitle: add-survey-suite radial-sideshots scenario
- sourceEdition: 2026-10-13
- sourceLocator: COGO inverse and forward requirement
- independent: no
- inputs: station (5,000 ft, 10,000 ft); CP 12 at N 35°15′20″ E for 185.42 ft; EP 4 at S 70° E for 240 ft; TBM at azimuth 270° for 95.5 ft
- outputs: CP 12 (5,151.411 ft, 10,107.029 ft); EP 4 (4,917.915 ft, 10,225.526 ft); TBM (5,000.000 ft, 9,904.500 ft)
- tolerance: 1 part in 100 billion, with 1e-9 ft absolute allowance in the golden vector
- verifiedBy: golden vector v001 and `radial_sideshots_share_the_forward_result`
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` evaluates 20 successful field books with Python's independent sine and cosine functions. The cases span cardinal and quadrant directions, DMS, decimal degrees, gons, coordinate signs, and three linear-unit families.
- `core/vectors/survey.cogo.radial-sideshots.jsonl` also holds five refusal cases for mixed feet, an invalid bearing, zero and negative distances, and an empty field book.

## Invariants

- A north shot changes only northing; east, south, and west follow the matching coordinate axis.
- Reversing a direction by 180° and keeping its distance makes an equal and opposite coordinate difference.
- Changing the station translates every calculated point by the same northing and easting offsets.
- Reordering the rows only reorders the output rows.
- Every result row is bit-for-bit identical to `survey.cogo.forward` called with the same station, direction, distance, and unit.
