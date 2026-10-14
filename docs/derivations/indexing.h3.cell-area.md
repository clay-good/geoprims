<!-- Derivation note (trust/correctness-program layer A). -->
# H3 cell area (`indexing.h3.cell-area`)

## Method

cellAreaKm2: the spherical excess of the cell's boundary polygon on H3's authalic sphere (radius 6,371.007180918475 km). The core computes it with h3o 0.11, the same library and calls as the general H3 tools.

## Equations

- cellAreaKm2 as defined in the H3 v4 API reference
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
- sourceLocator: `cell_area` and `average_hexagon_area`
- independent: yes
- inputs: 88ad361519fffff
- outputs: the cell's area, its resolution's average, and the ratio, as H3 C gives them
- tolerance: exact for indexes and counts; 1e-11° on angles; 1e-8 relative on areas and lengths
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_h3_views.py`: 20 or more cases from random points at all 16 resolutions, with pentagons included, each answered by H3 C 4.5.0 through h3-py rather than by h3o.
- `core/vectors/indexing.h3.cell-area.jsonl`: those vectors and refusal cases.
- `core/crates/gp-indexing/tests/indexing.rs` `focused_h3_operations_match_the_general_tools` requires the same answers as the general H3 tools.

## Invariants

- The areas of a cell's children add up to the cell's area to within the tolerance.
- The ratio is near 1 away from the pentagons.
- Areas are positive and below the sphere's.
