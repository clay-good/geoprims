<!-- Derivation note (trust/correctness-program layer A). -->
# Is this H3 cell a pentagon? (`indexing.h3.is-pentagon`)

## Method

isPentagon: a cell is a pentagon when its base cell is one and every digit below it is the center digit 0. The core computes it with h3o 0.11, the same library and calls as the general H3 tools.

## Equations

- isPentagon as defined in the H3 v4 API reference
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
- sourceLocator: `is_pentagon`
- independent: yes
- inputs: 830800fffffffff
- outputs: yes, with five neighbors, as H3 C gives them
- tolerance: exact for indexes and counts; 1e-11° on angles; 1e-8 relative on areas and lengths
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_h3_views.py`: 20 or more cases from random points at all 16 resolutions, with pentagons included, each answered by H3 C 4.5.0 through h3-py rather than by h3o.
- `core/vectors/indexing.h3.is-pentagon.jsonl`: those vectors and refusal cases.
- `core/crates/gp-indexing/tests/indexing.rs` `focused_h3_operations_match_the_general_tools` requires the same answers as the general H3 tools.

## Invariants

- Each resolution has exactly 12 pentagons.
- A pentagon's center child is a pentagon.
- A pentagon has 5 directed edges and 5 vertexes.
