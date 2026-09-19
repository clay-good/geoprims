<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# H3 edges and vertexes (`indexing.h3.edges`)

## Method

The directed edges from a cell to each neighbor (originToDirectedEdges), with each edge's neighbor and length, and the cell's vertexes (cellToVertexes).

## Equations

- One directed edge per neighbor: six for a hexagon, five for a pentagon.
- Edge length: the great-circle length of the shared boundary segment on the authalic sphere.

## Symbols and units

Cells, edges, and vertexes as H3 indexes; lengths in meters.

## Domain

Any valid cell.

## Approximations

None beyond floating point in the lengths.

## Worked example

- sourcePublisher: Uber Technologies and the H3 contributors
- sourceTitle: H3 C library via h3-py
- sourceEdition: H3 C 4.4.1 (h3-py 4.4.2)
- sourceLocator: originToDirectedEdges(892a8471487ffff) and cellToVertexes
- independent: yes
- inputs: cell 892a8471487ffff
- outputs: 6 directed edges and 6 vertexes
- tolerance: identical sets
- verifiedBy: automated differential test (below)
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/h3_family_parity.rs`: 300 seeded cells at every resolution, one in eight a pentagon, against H3 C 4.4.1 via h3-py 4.4.2 through the public tools (every edge and vertex, identical as sets)
- `tools/vectors/gen_h3_family_diff.py`: regenerates that fixture
- `core/vectors/indexing.h3.edges.jsonl`: 20 vectors from H3 C, run through the core on every build

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `h3_family_invariants`: hexagons have six edges and vertexes and pentagons five, and every edge leads to a neighbor
