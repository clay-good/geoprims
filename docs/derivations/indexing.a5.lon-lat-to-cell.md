<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# A5 cell from latitude and longitude (`indexing.a5.lon-lat-to-cell`)

## Method

A5 0.10.1 converts longitude and latitude to its authalic sphere, selects a dodecahedron face and quintant, projects into the face, and encodes the pentagonal cell through its Hilbert subdivision. The Rust core pins `a5` 0.10.1, the same release as the `a5-js` reference.

## Equations

The tool delegates the DSEA projection, pentagonal containment search, Hilbert encoding, center, and equal-area calculation to A5 0.10.1. Cell area is the authalic surface area divided by `12` at resolution 0 and by `60 × 4^(r − 1)` at resolutions 1–30.

## Symbols and units

`lat`, `lon`: degrees; `r`: requested resolution 0–30; cell: 64-bit hexadecimal A5 index; area: square kilometers.

## Domain

Every finite longitude and latitude on Earth and all 31 A5 resolutions. At resolution 30, cells in quintants 42–59 fall back to resolution 29 as A5 0.10.1 specifies.

## Approximations

A5 uses an authalic sphere. The cell index and equal area follow that model rather than an ellipsoidal cadastral surface.

## Worked example

- sourcePublisher: A5 project
- sourceTitle: A5 index encoding example
- sourceEdition: a5-js 0.10.1
- sourceLocator: lonLatToCell([-0.1276, 51.5074], 10)
- independent: no (canonical a5-js cross-language reference; the tool remains experimental)
- inputs: longitude -0.1276°, latitude 51.5074°, resolution 10
- outputs: 63611a8000000000
- tolerance: exact index
- verifiedBy: automated differential test
- verifiedOn: 2026-10-12

## Differential tests

- `core/crates/gp-indexing/tests/a5_parity.rs`: 120 fixed and seeded points across resolutions 0–30, the antimeridian, and both poles against a5-js 0.10.1; every index is identical and centers agree within 1e-10°.

## Invariants

- Every returned index decodes to the actual resolution and its center indexes back to the same cell.
- All cells at one resolution report the same area.
