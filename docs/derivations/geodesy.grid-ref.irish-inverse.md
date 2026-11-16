<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Irish Grid reference to latitude and longitude (`geodesy.grid-ref.irish-inverse`)

## Method

This reads an Irish Grid reference and returns the square it names. The letter gives the 100 km square (see the forward tool's table), and the figures give the offset inside it, so a reference names a square, not a point: two figures a 10 km square, six a 100 m square, ten a meter. The tool returns the square's south-west corner and size, and the latitude and longitude of its center, half a square further east and north. The center is unprojected from EPSG 29903 onto TM75 and, when WGS 84 is asked for, carried there by EPSG operation 1954.

Spaces and case do not matter: `O 15 34`, `o1534`, and `O 15 34` are one reference.

## Equations

- The letter's index `i` in `ABCDEFGHJKLMNOPQRSTUVWXYZ`: `E₀ = (i mod 5)·100 km`, `N₀ = (4 − ⌊i/5⌋)·100 km`.
- With `d` figures each: side `s = 10^(5−d)` m, `E = E₀ + figures_E·s`, `N = N₀ + figures_N·s`.
- Center `(E + s/2, N + s/2)`, then the inverse transverse Mercator and the Helmert transformation reversed.

## Symbols and units

The reference as text; easting and northing of the south-west corner and the square's side in meters; the center's latitude and longitude in degrees on WGS 84 or TM75.

## Domain

- One letter, not I, then an even number of figures up to ten. Anything else is refused with `INVALID_INPUT` naming what is wrong.
- A center outside EPSG's area for the grid carries `OUTSIDE_AREA_OF_USE`.

## Approximations

The square's corner and size are exact. On WGS 84 the latitude and longitude carry EPSG 1954's 1 m, the `LOW_ACCURACY_TRANSFORM` warning.

## Worked example

- sourcePublisher: IOGP
- sourceTitle: TM75 / Irish Grid (EPSG 29903)
- sourceEdition: EPSG v13.102, as shipped in PROJ 9.9.0
- sourceLocator: The grid's lettering of its 100 km squares: O runs from E 300000 to 400000 and N 200000 to 300000
- independent: yes
- inputs: reference O 15 34, datum WGS84
- outputs: south-west corner E 315000 m N 234000 m, a 1,000 m square
- tolerance: exact
- verifiedBy: `core/crates/gp-geo/src/gridref.rs` `letters_squares_and_round_trips` and golden vectors v001 to v020
- verifiedOn: 2026-11-16

## Differential tests

- `tools/vectors/gen_irish.py` builds 20 of the 22 vectors in `core/vectors/geodesy.grid-ref.irish-inverse.jsonl`: the six-figure reference of each of ten places, lettered by the script's own lettering, read on WGS 84 and on TM75, with the square's corner exact and its center carried back by PROJ 9.9.0 through EPSG 29903 and EPSG 1954 within a nanodegree.
- The last two are the convention's own cases: N 1 3, a 10 km square, and the letter I, refused.

## Invariants

- `core/crates/gp-geo/src/gridref.rs` `letters_squares_and_round_trips`: every square at every precision decodes to a corner that encodes back to the same reference, and the letter I is refused.
- `core/crates/gp-search/src/detect.rs` `british_and_irish_grid_references`: a pasted reference, spaced or not, reaches this tool.
