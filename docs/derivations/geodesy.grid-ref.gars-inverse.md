<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# GARS cell to position (`geodesy.grid-ref.gars-inverse`)

## Method

Read the band number, the two band letters, and the optional quadrant and keypad digits back into the cell's south-west corner and size, as in GeographicLib's GARS class. The result is the cell's bounds and center.

## Equations

- west = −180 + (band − 1)/2, south = −90 + (24 × letter1 + letter2)/2 (letters without I and O).
- Quadrant q: west += ((q − 1) mod 2) × 15′, south += (1 − ⌊(q − 1)/2⌋) × 15′.
- Keypad k: west += ((k − 1) mod 3) × 5′, south += (2 − ⌊(k − 1)/3⌋) × 5′.

## Symbols and units

Bounds and center in degrees; the cell is 30′, 15′, or 5′ square.

## Domain

Five to seven characters: a band 001 to 720, letters AA to QZ, a quadrant 1 to 4, and a keypad 1 to 9. Letters may be lowercase. Anything else is refused with INVALID_INPUT.

## Approximations

None.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: Global Area Reference System (GARS) description
- sourceEdition: earth-info.nga.mil page of October 6, 2006
- sourceLocator: Design: band 001 is 180° to 179°30′W and band AA is 90°S to 89°30′S; band 002 is 179°30′W to 179°W and AB is 89°30′S to 89°S
- independent: yes
- inputs: 001AA and 002AB
- outputs: south-west corners (−90, −180) and (−89.5, −179.5); centers (−89.75, −179.75) and (−89.25, −179.25)
- tolerance: 1e-12°
- verifiedBy: golden vectors v024 and v025, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geo/tests/gridref_diff.rs`: 1,218 GARS encodings and decodes identical to GeographicLib 2.7's C++ GARS class (poles and the antimeridian included)
- `core/crates/gp-geodesy/tests/gridref_parity.rs`: `grid_references_match_python_libraries` checks 500 more points at all three precisions, through the tools, against pygeodesy 26.9.9's separately written gars module
- `tools/vectors/gen_gridref_diff.py` and `tools/vectors/gen_gridref.py`: regenerate those fixtures
- `core/vectors/geodesy.grid-ref.gars-inverse.jsonl`: 25 vectors, including lowercase codes and two refusals

## Invariants

- `core/crates/gp-geodesy/tests/gridref.rs` `grid_reference_invariants`: at every precision the cell holds the point, its center encodes back to the same code, and each finer cell lies inside the coarser one
