<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Geohash encoder (`indexing.geohash.encode`)

## Method

Bisect longitude and latitude in turn, starting with longitude. Each halving adds one bit: 1 for the upper half, 0 for the lower. The interleaved bits are read five at a time as base-32 digits from the alphabet `0123456789bcdefghjkmnpqrstuvwxyz` (no a, i, l, or o). The cell is the final pair of intervals. Its width and height in meters are given on the mean-radius sphere.

## Equations

- Bits for precision p: 5p in total, ⌈5p/2⌉ for longitude and ⌊5p/2⌋ for latitude.
- Cell size: 360° / 2^⌈5p/2⌉ of longitude by 180° / 2^⌊5p/2⌋ of latitude.
- Height ≈ R1 × Δφ (radians), width ≈ R1 × cos φ × Δλ (radians), R1 = 6,371,008.771 m.

## Symbols and units

φ latitude and λ longitude (degrees, WGS 84 as given), p precision (1–12 characters), R1 the IUGG mean radius. Bounds are in degrees; cell sizes in meters.

## Domain

Latitude −90° to 90°, longitude −180° to 180°, precision 1 to 12. A point exactly on a cell edge goes to the upper (north or east) cell, as in the reference implementations.

## Approximations

None in the code: bisecting by powers of two is exact in binary floating point. Only the cell sizes in meters are approximate (spherical, within 0.5%).

## Worked example

- sourcePublisher: Wikipedia contributors
- sourceTitle: Geohash (encoding example, after Niemeyer's geohash.org)
- sourceEdition: retrieved 2026-09-19
- sourceLocator: Example section: 57.64911, 10.40744 is u4pruydqqvj
- independent: yes
- inputs: lat 57.64911, lon 10.40744, precision 11
- outputs: u4pruydqqvj
- tolerance: exact string
- verifiedBy: golden vector v023, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/dev_parity.rs`: 1,000 random points at every precision against pygeohash 3.3.2, a separately written library, comparing the geohash exactly and the cell bounds within 1e-12°
- `tools/vectors/gen_dev_diff.py`: regenerates that fixture
- `core/vectors/indexing.geohash.encode.jsonl`: 23 vectors from a Python implementation of the algorithm, including poles, the antimeridian, and the origin

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `geohash_encode_invariants`: every cell holds its point, shorter geohashes are prefixes of longer ones, and each added character splits the cell into 32
