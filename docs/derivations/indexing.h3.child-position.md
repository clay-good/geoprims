<!-- Derivation note (trust/correctness-program layer A). -->
# H3 child position (`indexing.h3.child-position`)

## Method

cellToChildPos: the cell's index in its ancestor's children in H3's own order, skipping the deleted K-axis subsequences under a pentagon. The core computes it with h3o 0.11, the same library and calls as the general H3 tools.

## Equations

- cellToChildPos as defined in the H3 v4 API reference
- Coordinates on H3's authalic sphere of radius 6,371.007180918475 km

## Symbols and units

Cells, edges, and vertexes are 64-bit H3 indexes written in hexadecimal. Angles in degrees, lengths in meters, areas in square kilometers.

## Domain

A valid H3 index of the kind the operation takes, at resolutions 0 to 15. Invalid text is refused at its input with INVALID_INPUT, except by the validator, which reports it.

## Approximations

None beyond H3's own model. h3o and H3 C agree within 1e-12° below 88° latitude, 5e-11° nearer the poles, and within 2e-8 relative on areas and lengths.

## Worked example

- sourcePublisher: Uber Technologies and the H3 contributors
- sourceTitle: H3 C library, through the h3-py bindings
- sourceEdition: H3 4.5.0
- sourceLocator: `cell_to_child_pos` and `cell_to_children`
- independent: yes
- inputs: 8a2a1072b59ffff under resolution 7
- outputs: its position among the 343 children, as H3 C gives them
- tolerance: exact for indexes and counts; 1e-11° on angles; 1e-8 relative on areas and lengths
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_h3_views.py`: 20 or more cases from random points at all 16 resolutions, with pentagons included, each answered by H3 C 4.5.0 through h3-py rather than by h3o.
- `core/vectors/indexing.h3.child-position.jsonl`: those vectors and refusal cases.
- `core/crates/gp-indexing/tests/indexing.rs` `focused_h3_operations_match_the_general_tools` requires the same answers as the general H3 tools.

## Invariants

- The center child is at position 0.
- Positions are below the ancestor's child count.
- childPosToCell of the position returns the cell.
