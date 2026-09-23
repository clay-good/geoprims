<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Tiles covering a box (`indexing.tile.cover`)

## Method

Given a latitude and longitude box and a zoom, this lists every Web Mercator tile that touches it — the set you would pre-cache, download or invalidate for that area.

The Web Mercator grid is 2^z tiles square. Longitude maps to a column linearly; latitude maps to a row through the Mercator projection, which is why the rows are not evenly spaced in latitude and why the grid stops at ±85.0511°, the latitude where the projection reaches a square. A box extending past that is clamped to it, because there are no tiles beyond.

Two details decide whether the answer is right at the edges. First, a box edge that lands exactly on a tile edge does **not** take the next tile: the east edge and the south edge are exclusive, so a box of exactly one tile returns one tile and not four. Second, a box whose west is greater than its east crosses the antimeridian, and the columns wrap through the end of the grid and back to zero rather than running backwards.

The result is capped: a request that would return an unreasonable number of tiles fails with `LIMIT_EXCEEDED` rather than building a list nobody wanted.

## Equations

- Column: x = ⌊(lon + 180) / 360 · 2^z⌋, with an east edge exactly on a boundary taking the tile to its west.
- Row: y = ⌊(1 − asinh(tan φ) / π) / 2 · 2^z⌋, with φ clamped to ±85.0511287798°.
- Columns run x(west) … x(east), wrapping through 2^z − 1 to 0 when west > east.
- Rows run y(north) … y(south), since y counts from the north.

## Symbols and units

`south`, `west`, `north` and `east` are degrees; `zoom` is the level. Out come `count`, the `x_range` and `y_range`, and the `tiles` themselves.

## Domain

Any box, any zoom the cap allows. Latitudes beyond ±85.0511° are clamped rather than refused, because the grid genuinely ends there. West greater than east means the box crosses the antimeridian.

## Approximations

None in the tile arithmetic, which is exact integer flooring of an exact projection. The projection itself is spherical Web Mercator, which is what the tile grid is defined on — using an ellipsoidal Mercator here would give a different and wrong tile.

## Worked example

- sourcePublisher: OpenStreetMap contributors
- sourceTitle: Slippy map tilenames
- sourceEdition: OSM wiki, current
- sourceLocator: the lon/lat to tile-number formulas, and the ±85.0511287798° limit of the square grid
- independent: yes
- inputs: 21 boxes, including one that is a single tile, one on exact tile edges, three that cross the antimeridian, one spanning the equator and prime meridian, one past the southern Mercator clamp, the whole world at zoom 1, and a request over the cap
- outputs: the tile count and the first and last tile in each case
- tolerance: exact
- verifiedBy: golden vectors v001 to v021, run by the core on every build
- verifiedOn: 2026-09-23

The expected tiles come from `tools/vectors/gen_cover.py` and `gen_cover_more.py`, which transcribe the slippy-map formulas in Python independently of the core, including the exclusive-edge rule and the antimeridian wrap. The box on exact tile edges is the case that catches an off-by-one: a naive floor on the east edge takes an extra column every time.

## Differential tests

- `tools/vectors/gen_cover.py` and `gen_cover_more.py`: all 21 vectors, from an independent Python transcription
- `core/crates/gp-indexing/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/indexing.tile.cover.jsonl`: 21 vectors from a single tile to the whole world

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `tile_cover_invariants`: the count equals the width of the column range times the height of the row range, so no tile is listed twice or missed; a box of exactly one tile returns exactly one, which is the exclusive-edge rule stated at its smallest; every returned tile's bounds actually intersect the box, checked through `indexing.tile.bounds` rather than recomputed here; one zoom deeper returns four times as many tiles for a box on tile boundaries; a box crossing the antimeridian returns the same tiles as the two boxes either side of it put together; and a box past ±85.0511° returns the same tiles as one clamped to it
