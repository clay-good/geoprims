<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Geohash decoder (`indexing.geohash.decode`)

## Method

Read each base-32 character (alphabet `0123456789bcdefghjkmnpqrstuvwxyz`) as five bits and replay the encoder's bisection: bits alternate longitude, latitude, starting with longitude, and each bit keeps the upper (1) or lower (0) half of the interval. The final intervals are the cell. The reported point is the cell's center, and the errors are half the cell's height and width.

## Equations

- Bits for p characters: 5p, ⌈5p/2⌉ for longitude and ⌊5p/2⌋ for latitude.
- Center: φ = (south + north) / 2, λ = (west + east) / 2.
- Errors: Δφ/2 = 90° / 2^⌊5p/2⌋, Δλ/2 = 180° / 2^⌈5p/2⌉.

## Symbols and units

φ latitude and λ longitude in degrees; p the number of characters (1–12). Errors and bounds are in degrees.

## Domain

Geohashes of 1 to 12 characters from the base-32 alphabet, in either case. Letters outside the alphabet (a, i, l, o) are refused with INVALID_INPUT, naming the first bad character.

## Approximations

None. Halving by powers of two is exact in binary floating point, so the bounds are exact.

## Worked example

- sourcePublisher: Wikipedia contributors
- sourceTitle: Geohash (decoding example)
- sourceEdition: retrieved 2026-09-19
- sourceLocator: Algorithm and example section: ezs42 decodes to 42.605, −5.603 with an error of ±0.022°
- independent: yes
- inputs: geohash ezs42
- outputs: 42.605°, −5.603° (the page's rounding of 42.60498°, −5.60303°)
- tolerance: 5e-4° (the page prints three decimals)
- verifiedBy: golden vector v024, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/dev_parity.rs`: `geohash_decode_matches_pygeohash` decodes 1,000 random geohashes (every precision) and compares the cell bounds with pygeohash 3.3.2 within 1e-12°
- `tools/vectors/gen_dev_diff.py`: regenerates that fixture
- `core/vectors/indexing.geohash.decode.jsonl`: 24 vectors, including the poles, the antimeridian, and a refused letter

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `geohash_decode_and_neighbor_invariants`: the decoded center lies in the cell and encodes back to the same geohash, and the errors are half the cell
