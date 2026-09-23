<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# H3 cell set outline (`indexing.h3.cells-to-polygon`)

## Method

Every cell contributes the segments of its boundary. A segment between two cells of the set is walked once from each side, so it appears twice and is dropped; what is left is the outline, and following those segments end to end traces it. Cells all wind the same way, so a ring wound like them is an outer ring and a ring wound against them is a hole, which is what tells the two apart without any containment test; a hole is then given to the smallest ring that contains it.

Written on h3o's core API rather than its geo feature, which breaks reproducible builds.

## Equations

- An edge is kept when the set contains it once: |{c ∈ S : e ∈ ∂c}| = 1.
- Twice the signed area of a ring, longitudes unwrapped so a ring across the antimeridian keeps its sign: 2A = Σ (λᵢ·φⱼ − λⱼ·φᵢ) over consecutive pairs. Its sign against the majority sign separates outer rings from holes.
- A hole belongs to the containing ring of least |A|, containment by ray casting with longitudes taken relative to the test point.

## Symbols and units

S is the set of cells, ∂c the boundary of cell c, φ latitude and λ longitude in degrees. Rings are closed: the first point repeats as the last. Points are snapped together within 1e-9°, which is far below the spacing of distinct H3 boundary points and far above the ~1e-12° by which two cells disagree about a point they share.

## Domain

Any set of H3 cells at one resolution, from a single cell to the 10,000 a compacted answer can carry. Mixed resolutions are refused: cells at different resolutions share no whole edges, so nothing would cancel and the trace would be wrong rather than merely coarse.

## Approximations

None in the topology: the set of kept edges is exact, and rings close by construction. The boundary is taken rather than the cell's five or six vertexes because a cell near one of the twelve pentagons carries extra points where its edge crosses an icosahedron face — at resolution 5 such a cell has seven boundary points and the pentagon itself ten — and a first version built on vertexes traced 25 points where H3 C traces 30. Edges are straight in latitude and longitude, as H3 draws cell boundaries, not geodesics.

## Worked example

- sourcePublisher: Uber Technologies and the H3 contributors
- sourceTitle: H3 `cellsToMultiPolygon`, through h3-py `cells_to_h3shape`
- sourceEdition: H3 C 4.4.1 via h3-py 4.4.2
- sourceLocator: `cells_to_h3shape` on the resolution 9 cell 892a8471487ffff with its six neighbours
- independent: yes
- inputs: the seven cells 892a8471483ffff, 892a8471487ffff, 892a847148fffff, 892a8471497ffff, 892a84714b3ffff, 892a84714bbffff, 892a847334bffff
- outputs: 1 polygon, 0 holes, 19 ring points (18 sides and the closing point), area 0.7373245575156182 km²
- tolerance: exact on the counts; 1e-9 relative on the area
- verifiedBy: golden vector v002, run by the core on every build
- verifiedOn: 2026-09-22

## Differential tests

- `tools/vectors/gen_h3_outline_diff.py`: 400 cell sets from H3 C through h3-py — solid patches, patches with a hole, separate pieces, sets around a pentagon, and sets across the antimeridian, at resolutions 3 to 11
- `core/crates/gp-indexing/tests/h3_outline_parity.rs`: those sets compared ring for ring, each ring turned to start at its lowest point so two tracings of one ring match however each began
- `core/vectors/indexing.h3.cells-to-polygon.jsonl`: 24 vectors from the same reference, run through the core on every build

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `outline_invariants`: one cell outlines as its own boundary; a disk outlines as one piece with no hole; filling the outline at the same resolution returns the set it came from, so outlining and filling are inverses; and a mixed-resolution set is refused
