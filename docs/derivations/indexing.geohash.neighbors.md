<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Geohash neighbors (`indexing.geohash.neighbors`)

## Method

Decode the geohash to its cell, step one cell height or width in each of the eight compass directions from the cell's center, and encode that point at the same precision. Longitude wraps across the antimeridian. A step north of the top row or south of the bottom row has no neighbor, and the tool says so ("none (past the pole)") instead of wrapping over the pole.

## Equations

- Cell height h = 180° / 2^⌊5p/2⌋, width w = 360° / 2^⌈5p/2⌉.
- Neighbor in direction (dy, dx): encode(φc + dy·h, wrap(λc + dx·w), p), with dy, dx ∈ {−1, 0, 1}.
- wrap(λ) = ((λ + 180) mod 360) − 180.

## Symbols and units

φc, λc the cell center (degrees), p the precision (characters), dy and dx the step in cells. Outputs are geohashes of the same length as the input.

## Domain

Any valid geohash of 1 to 12 characters. Cells on the top or bottom row have no neighbor across the pole in that direction (three of the eight are missing).

## Approximations

None. The neighbor's center lies strictly inside the neighboring cell, so encoding it is exact.

## Worked example

- sourcePublisher: Chris Veness (Movable Type Scripts)
- sourceTitle: latlon-geohash test suite (test.js)
- sourceEdition: latlon-geohash 2.0.0
- sourceLocator: "fetches neighbours": ezzz has n gbpb, ne u000, e spbp, se spbn, s ezzy, sw ezzw, w ezzx, nw gbp8
- independent: yes
- inputs: geohash ezzz
- outputs: gbpb, u000, spbp, spbn, ezzy, ezzw, ezzx, gbp8
- tolerance: exact strings
- verifiedBy: golden vector v024, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/dev_parity.rs`: `geohash_neighbors_match_pygeohash` compares the four edge neighbors of 500 random geohashes (one in ten on a polar row) with pygeohash 3.3.2 get_adjacent, including the missing neighbor past the pole
- `tools/vectors/gen_dev_diff.py`: regenerates that fixture
- `core/vectors/indexing.geohash.neighbors.jsonl`: 24 vectors, including the antimeridian and both poles

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `geohash_decode_and_neighbor_invariants`: each neighbor shares an edge with the cell, and the cell is its neighbor's opposite neighbor
