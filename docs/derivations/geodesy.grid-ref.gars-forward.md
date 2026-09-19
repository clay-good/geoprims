<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# GARS cell for a point (`geodesy.grid-ref.gars-forward`)

## Method

NGA's Global Area Reference System, as in GeographicLib's GARS class. Three digits number the 30-minute longitude band eastward from 180° (001 to 720); two letters name the 30-minute latitude band northward from 90° S (AA to QZ, without I and O). A quadrant digit (1 to 4 from the north-west) gives 15 minutes, and a keypad digit (1 to 9 from the north-west) gives 5 minutes.

## Equations

- Longitude band: ⌊(λ + 180) × 2⌋ + 1; latitude band: ⌊(φ + 90) × 2⌋, written as two letters in base 24.
- Quadrant: 1 + (column) + 2 × (1 − row), with column and row the 15′ halves.
- Keypad: 1 + (column) + 3 × (2 − row), with column and row the 5′ thirds.

## Symbols and units

φ latitude and λ longitude in degrees (WGS 84); precision 30min, 15min, or 5min.

## Domain

Latitude −90° to 90°, longitude −180° to 180° (180° E is written as 180° W, band 001). +90° falls in the last band, QZ.

## Approximations

None.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: Global Area Reference System (GARS) description
- sourceEdition: earth-info.nga.mil page of October 6, 2006
- sourceLocator: Design: "180 E to 179 30'W is band 001; 179 30'W to 179 00'W is band 002"; "90 00'S to 89 30'S is band AA; 89 30'S to 89 00'S is band AB"
- independent: yes
- inputs: −89.75, −179.75 and −89.25, −179.25 at 30min
- outputs: 001AA and 002AB
- tolerance: exact strings
- verifiedBy: golden vectors v022 and v023, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geo/tests/gridref_diff.rs`: 1,218 GARS encodings and decodes identical to GeographicLib 2.7's C++ GARS class (poles and the antimeridian included)
- `core/crates/gp-geodesy/tests/gridref_parity.rs`: `grid_references_match_python_libraries` checks 500 more points at all three precisions, through the tools, against pygeodesy 26.9.9's separately written gars module
- `tools/vectors/gen_gridref_diff.py` and `tools/vectors/gen_gridref.py`: regenerate those fixtures
- `core/vectors/geodesy.grid-ref.gars-forward.jsonl`: 23 vectors

## Invariants

- `core/crates/gp-geodesy/tests/gridref.rs` `grid_reference_invariants`: at every precision the cell holds the point, its center encodes back to the same code, and each finer cell lies inside the coarser one
