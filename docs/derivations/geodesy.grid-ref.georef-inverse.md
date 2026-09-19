<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# GEOREF to position (`geodesy.grid-ref.georef-inverse`)

## Method

Read the tile letters, degree letters, and minute digits (split evenly between longitude and latitude) back into the cell's south-west corner and size, as in GeographicLib's Georef class. The result is the cell's bounds and center.

## Equations

- west = −180 + 15 × tile_lon + degree_lon + m_lon / (60 × 10^(n−2)).
- south = −90 + 15 × tile_lat + degree_lat + m_lat / (60 × 10^(n−2)).
- Cell size: 15°, 1°, or 1′/10^(n−2) square.

## Symbols and units

n digits of minutes per coordinate (2 to 6 here); bounds and center in degrees.

## Domain

2 or 4 letters, then an even number of digits with minutes below 60 (NKLN6099 is refused). Letters may be lowercase. Anything else is refused with INVALID_INPUT.

## Approximations

None.

## Worked example

- sourcePublisher: Wikipedia contributors
- sourceTitle: World Geographic Reference System (Example)
- sourceEdition: retrieved 2026-09-19
- sourceLocator: NAS Patuxent River (38.286108, −76.4291704) is at GJPJ3417, to the nearest minute
- independent: yes
- inputs: GJPJ3417
- outputs: the 1′ cell 38°17′ to 38°18′ N, 76°26′ to 76°25′ W, which holds the station
- tolerance: half a minute (1/120°) between the cell's center and the station
- verifiedBy: golden vector v022, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geo/tests/gridref_diff.rs`: 5,278 GEOREF encodings and decodes identical to GeographicLib 2.7's C++ Georef class, every precision
- `core/crates/gp-geodesy/tests/gridref_parity.rs`: `grid_references_match_python_libraries` checks 500 more points at all seven precisions, through the tools, against pygeodesy 26.9.9's separately written wgrs module
- `tools/vectors/gen_gridref_diff.py` and `tools/vectors/gen_gridref.py`: regenerate those fixtures
- `core/vectors/geodesy.grid-ref.georef-inverse.jsonl`: 22 vectors

## Invariants

- `core/crates/gp-geodesy/tests/gridref.rs` `grid_reference_invariants`: at every precision the cell holds the point, its center encodes back to the same code, and each finer cell lies inside the coarser one
