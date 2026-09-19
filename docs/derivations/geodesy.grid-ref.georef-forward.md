<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# GEOREF for a point (`geodesy.grid-ref.georef-forward`)

## Method

The World Geographic Reference System, as in GeographicLib's Georef class. Two letters name the 15° tile (24 longitude letters from 180° W, 12 latitude letters from 90° S, without I and O), two more the 1° square inside it (15 letters each), then minutes of longitude and of latitude east and north of the square's corner, with more digits for finer precision.

## Equations

- Tile: ⌊(λ + 180)/15⌋, ⌊(φ + 90)/15⌋; degree: the remainders' whole degrees.
- Minutes at n digits per coordinate: ⌊(λ + 180) × 60 × 10^(n−2)⌋ mod (60 × 10^(n−2)), and the same for latitude.

## Symbols and units

φ latitude and λ longitude in degrees (WGS 84); precision 15deg, 1deg, 1min, 0.1min, 0.01min, 0.001min, or 0.0001min.

## Domain

Latitude −90° to 90°, longitude −180° to 180°. 180° E is written as 180° W (GeographicLib indexes one tile past the end there); +90° falls in the last row.

## Approximations

None: minutes are truncated, never rounded.

## Worked example

- sourcePublisher: Wikipedia contributors
- sourceTitle: World Geographic Reference System (Example)
- sourceEdition: retrieved 2026-09-19
- sourceLocator: "Naval Air Station Patuxent River (38°17′10″N 76°24′42″W) / (38.286108, −76.4291704) is located (to the nearest minute) at position GJPJ3417"
- independent: yes
- inputs: lat 38.286108, lon −76.4291704, precision 1min
- outputs: GJPJ3417
- tolerance: exact string
- verifiedBy: golden vector v021, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geo/tests/gridref_diff.rs`: 5,278 GEOREF encodings and decodes identical to GeographicLib 2.7's C++ Georef class, every precision
- `core/crates/gp-geodesy/tests/gridref_parity.rs`: `grid_references_match_python_libraries` checks 500 more points at all seven precisions, through the tools, against pygeodesy 26.9.9's separately written wgrs module
- `tools/vectors/gen_gridref_diff.py` and `tools/vectors/gen_gridref.py`: regenerate those fixtures
- `core/vectors/geodesy.grid-ref.georef-forward.jsonl`: 21 vectors

## Invariants

- `core/crates/gp-geodesy/tests/gridref.rs` `grid_reference_invariants`: at every precision the cell holds the point, its center encodes back to the same code, and each finer cell lies inside the coarser one
