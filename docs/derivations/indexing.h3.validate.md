<!-- Derivation note (trust/correctness-program layer A). -->
# Validate an H3 index (`indexing.h3.validate`)

## Method

isValidCell: the high bit is 0, the mode is 1 (cell), the reserved bits are 0, the base cell is under 122, every digit below the resolution is 0–6 and every one past it is 7, and a pentagon's first non-zero digit is not 1 (the deleted K axis). The core computes it with h3o 0.11, the same library and calls as the general H3 tools.

## Equations

- isValidCell as defined in the H3 v4 API reference
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
- sourceLocator: `is_valid_cell`
- independent: yes
- inputs: 8f2a1072b59a5f7 and 9 valid cells
- outputs: valid or not, with the reason, as H3 C gives them
- tolerance: exact for indexes and counts; 1e-11° on angles; 1e-8 relative on areas and lengths
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_h3_views.py`: 20 or more cases from random points at all 16 resolutions, with pentagons included, each answered by H3 C 4.5.0 through h3-py rather than by h3o.
- `core/vectors/indexing.h3.validate.jsonl`: those vectors and refusal cases.
- `core/crates/gp-indexing/tests/indexing.rs` `focused_h3_operations_match_the_general_tools` requires the same answers as the general H3 tools.

## Invariants

- `core/crates/gp-indexing/tests/h3_views_invariants.rs` `h3_views_invariants`, on hexagons at resolutions 0, 3, 5, 9, and 12 and on pentagons: every cell another tool returns is valid, and a directed-edge index is not a valid cell.
