<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# S2 cell for a point (`indexing.s2.lat-lng-to-cell`)

## Method

S2 wraps the sphere in a cube. A point becomes a unit vector, the vector picks the cube face it points through, and the face carries two coordinates u and v. Those are straightened by a quadratic so that cells come out closer to equal in area than the raw projection gives, turned into integers i and j on a grid of 2³⁰ by 2³⁰, and woven together along a Hilbert curve. The curve position, the face, and the level pack into one 64-bit integer, which is what an S2 cell id is: cells near each other on the sphere have ids near each other, which is the property the whole scheme exists for.

## Equations

- Face and (u, v): the largest component of the unit vector chooses the face; the other two, divided by it, give u and v in [−1, 1].
- The quadratic straightening, s from u: s = ½·√(1 + 3u) for u ≥ 0, and 1 − ½·√(1 − 3u) otherwise, with the inverse u = ⅓(4s² − 1) for s ≥ ½.
- Integer grid: i = ⌊s · 2³⁰⌋, clamped to 2³⁰ − 1, and the same for j.
- Hilbert: the (i, j) pair is walked bit by bit through the curve's four orientations, giving a position along the curve.
- The id is face, then the curve position, then a single 1 bit marking the level; the token is its hexadecimal with trailing zeros removed.

## Symbols and units

Latitude and longitude in degrees. Face is 0 to 5, level 0 to 30, where level 30 cells are about 1 cm across and level 0 is a sixth of the sphere. The token is a lower-case hexadecimal string. The cell id is a 64-bit unsigned integer, returned as a string because it does not fit a double.

## Domain

Any latitude and longitude, at any level from 0 to 30. There is no gap and no overlap: every point on the sphere falls in exactly one cell at each level.

## Approximations

The cell id is exact integer arithmetic, so it either agrees with another implementation or does not — there is no tolerance in it. The quadratic straightening is S2's own definition rather than an approximation of something else. The centre and corners returned with the cell carry ordinary floating-point rounding, which shows at the 1e-9 degree level against an independent implementation.

## Worked example

- sourcePublisher: s2sphere contributors
- sourceTitle: s2sphere, a pure-Python implementation of the S2 geometry library
- sourceEdition: s2sphere 0.2.5
- sourceLocator: `CellId.from_lat_lng(LatLng.from_degrees(40.446111, -79.982222)).parent(15).to_token()`, run for this note
- independent: yes
- inputs: lat 40.446111, lon −79.982222, level 15
- outputs: token 8834f3dec, id 9814737625976668160, face 4, centre 40.44655064325857°, −79.98221257194216°
- tolerance: exact on the token, the id, the face, and the level; 1e-9 degrees on the centre
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-22

## Differential tests

- `tools/vectors/gen_s2.py`: 400 cases against s2sphere spread over the sphere and every level, comparing tokens, levels, hierarchy, and edge neighbours exactly, and geometry within 10 nanometres
- `core/crates/gp-indexing/tests/s2_parity.rs`: that fixture, run on every build
- `core/vectors/indexing.s2.lat-lng-to-cell.jsonl`: 22 vectors over every cube face, both poles, the antimeridian, and levels 4 to 28

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `s2_invariants`: over six places and six levels, a point's cell at one level is the parent of its cell at the next, the token survives the trip out and back, each of the four children names this cell as its parent, and the four neighbours are distinct cells at the same level that each count this one among their own
