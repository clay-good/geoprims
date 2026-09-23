<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# S2 cell details (`indexing.s2.cell-info`)

## Method

Everything a cell id carries, read back out of it. The id is unpacked into its face, its position along the Hilbert curve, and the level marked by its lowest set bit. The centre and the four corners come from inverting the projection: curve position back to i and j, those back to s and t, the quadratic undone to u and v, and the face's axes turned back into a unit vector. The parent is the id with the level bit moved out; the four children are the id with it moved in, at each of the four curve positions.

## Equations

- Level: 30 minus half the number of trailing zero bits below the level bit.
- Face: the three highest bits of the id.
- Inverse projection: u = ⅓(4s² − 1) for s ≥ ½, and the mirror below, then the vector (1, u, v) on the face's axes, normalised.
- Parent at level L: the id with the level bit set at L and everything below it cleared.
- Exact area: the spherical excess of the four corners, by Girard's theorem on the two triangles the cell's diagonal makes.

## Symbols and units

The cell is given as a token or a decimal id. Level 0 to 30; face 0 to 5. Centre and corners in degrees. Area in square metres, from the spherical excess scaled by the Earth's mean radius.

## Domain

Any valid S2 cell id. A cell at level 30 has no children, and a face cell at level 0 has no parent; both say so rather than inventing one.

## Approximations

The hierarchy and the token are exact integer work. The geometry inherits the projection's floating point: against an independent implementation the centres and corners agree within 10 nanometres on the ground, and the exact area agrees to rounding down to level 20, drifting to about 2e-7 relative at level 30, which is the precision of the corner vectors themselves rather than of the area formula.

## Worked example

- sourcePublisher: s2sphere contributors
- sourceTitle: s2sphere, a pure-Python implementation of the S2 geometry library
- sourceEdition: s2sphere 0.2.5
- sourceLocator: `Cell(CellId.from_token('8834f3dec'))` with `get_center`, `get_vertex`, and `parent`, run for this note
- independent: yes
- inputs: cell 8834f3dec
- outputs: level 15, face 4, centre 40.44655064325857°, −79.98221257194216°, parent 8834f3df
- tolerance: exact on the token, level, face, and parent; 1e-9 degrees on the centre
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-22

## Differential tests

- `tools/vectors/gen_s2.py`: 400 cases against s2sphere over the sphere and every level, comparing tokens, levels, hierarchy, and edge neighbours exactly, geometry within 10 nanometres, and exact areas to rounding down to level 20
- `core/crates/gp-indexing/tests/s2_parity.rs`: that fixture, run on every build
- `core/vectors/indexing.s2.cell-info.jsonl`: 22 vectors carrying the centre, the four corners, the parent, and all four children

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `s2_invariants`: over six places and six levels, the token survives the trip out and back, each of the four children names this cell as its parent and sits one level finer, and the cell one level coarser for the same point is this cell's parent
