<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# British National Grid reference to latitude and longitude (`geodesy.grid-ref.bng-inverse`)

## Method

A grid reference names a square, not a point, and the size of that square is in the number of figures: two figures name a 10 km square, four a 1 km square, six 100 m, eight 10 m, ten a metre. The tool answers with the square itself — its south-west corner, its side, and its edges in latitude and longitude — and gives the center as the single latitude and longitude, because that is the point of the square a reader means when they ask "where is TQ 30 80".

The two letters give the 100 km square. The lettering runs in 500 km blocks from the square two columns west of the grid's false origin, with the letter I left out, so reading it back is the inverse of the encoding: from the first letter's index `l1` and the second's `l2`, the column is `((l1 − 2) mod 5)·5 + l2 mod 5` and the row is `19 − ⌊l1/5⌋·5 − ⌊l2/5⌋`. Squares outside the grid's seven columns and thirteen rows are refused by name.

The figures are an offset inside that square, and they were truncated when the reference was written, so they give the square's south-west corner. The center is half a square further east and north, unprojected from the National Grid transverse Mercator (Airy 1830, true origin 49°N 2°W, scale 0.999 601 271 7, false origin 400 km west and 100 km north) onto OSGB36, and then carried to WGS 84 by Ordnance Survey's published seven-parameter transformation, EPSG operation 1314, when WGS 84 is asked for.

## Equations

- Column and row of the 100 km square, from the letters, as above; its corner is `(column·100000, row·100000)`.
- Side of the square named by `f` figures: `100000/10^(f/2)`, and 100 km with no figures at all.
- South-west corner: the square's corner plus the figures times the side.
- Center: the corner plus half the side each way, then the inverse transverse Mercator, then the datum step if WGS 84 is asked for.
- Edges: the four corners through the same inverse, and the smallest and largest latitude and longitude of them.

## Symbols and units

Eastings and northings in meters from the National Grid's false origin; the square's side in meters; latitudes and longitudes in degrees on the datum asked for.

## Domain

- Two letters, neither of them I, naming a square inside the grid's seven columns and thirteen rows, else `INVALID_INPUT` saying which rule was broken.
- After the letters, an even number of figures, at most ten, all digits. Spaces anywhere are ignored.
- On WGS 84 the seven-parameter path has an area of use (Great Britain and the Isle of Man).

## Approximations

The square is exact; the latitude and longitude of its center are as good as the datum step, which on WGS 84 is the seven-parameter transformation Ordnance Survey states as about 2 m, hence `LOW_ACCURACY_TRANSFORM`. A reference's own precision is usually the larger error: a two-figure reference names a 10 km square, so its center is good to 5 km by construction, which is why the square's size is an output and not a footnote. Heights are untouched.

## Worked example

- sourcePublisher: Ordnance Survey
- sourceTitle: A Guide to Coordinate Systems in Great Britain
- sourceEdition: Version 3.6 (D00659), © Crown copyright 2020
- sourceLocator: Section 7.1, the National Grid reference convention: the grid is divided into 100 km squares with a two-letter code, and a reference is that code followed by an easting and a northing inside it
- independent: yes
- inputs: reference `TQ 30 80`
- outputs: a 1 km square whose south-west corner is E 530000 N 180000
- tolerance: exact
- verifiedBy: golden vector v001 and `core/crates/gp-geo/src/gridref.rs` `the_conventions_own_example`
- verifiedOn: 2026-11-07

## Differential tests

- `tools/vectors/gen_bng.py` builds the 10 vectors in `core/vectors/geodesy.grid-ref.bng-inverse.jsonl`: six references read back, from a bare `TQ` (a whole 100 km square) to a ten-figure reference (a metre), with their centers unprojected by PROJ 9.9.0 through `cs2cs` using the OS guide's National Grid parameters, and four refusals, each from a rule the guide states.
- The lettering is written again in Python in the same generator, so the encoding and the decoding are each checked against a separate implementation.

## Invariants

- `core/crates/gp-geo/src/gridref.rs` `the_conventions_own_example`: `TQ 30 80` decodes to the 1 km square at E 530000 N 180000; the letter I is refused by name; and every one of the grid's 100 km squares round trips through its own two-letter code, which is the encoding and the decoding checked against each other over the whole grid.
