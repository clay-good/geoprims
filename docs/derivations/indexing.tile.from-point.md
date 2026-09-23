<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Map tile for a point (`indexing.tile.from-point`)

## Method

Project the point with spherical Web Mercator (EPSG:3857) onto a square world of 2^z × 2^z tiles, then take the integer tile coordinates. The quadkey interleaves the bits of x and y as base-4 digits (Bing Maps). TMS numbers rows from the south. The ground resolution is the length of one pixel on the Web Mercator sphere at that latitude.

## Equations

- n = 2^z, x = ⌊(λ + 180)/360 × n⌋, y = ⌊(1 − asinh(tan φ)/π)/2 × n⌋.
- TMS y = n − 1 − y.
- Quadkey digit i (from the most significant) = bit_i(x) + 2 × bit_i(y).
- Ground resolution = cos φ × 2π × 6,378,137 m / (tile size × 2^z).

## Symbols and units

φ latitude and λ longitude (degrees), z zoom (0–30), tile size 256 or 512 px. Ground resolution in meters per pixel.

## Domain

Latitude is clamped to ±85.05112878° (the edge of the square Web Mercator map), and the WEB_MERCATOR_CLAMPED warning says so. Longitude −180° to 180°.

## Approximations

None for the tile: the tile grid is defined on the sphere. Web Mercator treats WGS 84 coordinates as spherical, so the ground resolution is exact on that sphere, not on the ellipsoid.

## Worked example

- sourcePublisher: Mapbox (mercantile) and Microsoft (Bing Maps Tile System)
- sourceTitle: mercantile README; Bing Maps Tile System
- sourceEdition: mercantile 1.2.1; Bing article of 2018-02-28
- sourceLocator: mercantile README, tile(*ul(486, 332, 10) + (10,)) is Tile(486, 332, 10); Bing, ground resolution 78,271.5170 m/px at level 1 on the equator
- independent: yes
- inputs: lat 53.33087298301705, lon −9.140625, zoom 10; and lat 0, lon 0, zoom 1
- outputs: 10/486/332; 78,271.5170 m per pixel
- tolerance: exact tile; 5e-5 m for the printed resolution
- verifiedBy: golden vectors v023 and v024, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/dev_parity.rs`: 1,000 random points at zooms 0 to 24 against mercantile 1.2.1 (tile and quadkey identical)
- `tools/vectors/gen_dev_diff.py`: regenerates that fixture
- `core/vectors/indexing.tile.from-point.jsonl`: vectors from the slippy-map formulas in Python, including zoom 0, the antimeridian, and the clamped poles, and v026, a tile size given as the number 512

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `tile_invariants`: the point lies in its tile's bounds, the quadkey has one digit per zoom level and extends its parent's, TMS y = 2^z − 1 − y, and the tile's center maps back to the tile
