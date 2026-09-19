<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Map tile bounds (`indexing.tile.bounds`)

## Method

Invert the Web Mercator tile grid: a tile's west and east edges are linear in x, and its north and south edges come from the inverse Gudermannian of its row edges. The tool reads `z/x/y` in XYZ or TMS order, or a quadkey. In detect mode, it shows the other reading when both are valid, because mixing up the two y conventions is the most common tile mistake.

## Equations

- n = 2^z, west = x/n × 360 − 180, east = (x + 1)/n × 360 − 180.
- north = atan(sinh(π(1 − 2y/n))), south = atan(sinh(π(1 − 2(y + 1)/n))), in degrees.
- TMS y = n − 1 − XYZ y. Quadkey digits are bit_i(x) + 2 × bit_i(y).

## Symbols and units

z zoom, x column, y row (XYZ counts rows from the north). Bounds and center in degrees.

## Domain

0 ≤ x, y < 2^z and z from 0 to 30. Any other tile is refused with INVALID_INPUT and the valid range.

## Approximations

None: exact inverse of the tile grid.

## Worked example

- sourcePublisher: Mapbox (mercantile) and Microsoft (Bing Maps Tile System)
- sourceTitle: mercantile README; Bing Maps Tile System
- sourceEdition: mercantile 1.2.1; Bing article of 2018-02-28
- sourceLocator: mercantile README, bounds(486, 332, 10) is west −9.140625, south 53.12040528310657, east −8.7890625, north 53.33087298301705; Bing, tile (3, 5) at level 3 has quadkey 213
- independent: yes
- inputs: tile 10/486/332; tile 3/3/5
- outputs: the bounds above; quadkey 213
- tolerance: 1e-9°; exact quadkey
- verifiedBy: golden vectors v025 and v026, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/dev_parity.rs`: the tiles of 1,000 random points at zooms 0 to 24 against mercantile 1.2.1 bounds (within 1e-11°) and quadkeys (identical)
- `tools/vectors/gen_dev_diff.py`: regenerates that fixture
- `core/vectors/indexing.tile.bounds.jsonl`: 26 vectors from the slippy-map formulas in Python, plus TMS, quadkey, and out-of-range cases

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `tile_invariants`: the bounds hold the point that produced the tile, and the center of the bounds maps back to the same tile
