<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# A5 cell boundary (`indexing.a5.cell-boundary`)

## Method

A5 0.10.1 decodes the 64-bit index, reconstructs its pentagon on the dodecahedron face, splits each face edge before projection, and maps the vertices back through the inverse equal-area and authalic projections. The ring is counterclockwise and closed.

## Equations

Automatic edge segmentation uses `max(1, 2^(6 − r))` segments per edge. Area is the authalic Earth area divided by the number of cells at resolution `r`; it is not estimated from the segmented ring.

## Symbols and units

Cell: 16-digit hexadecimal A5 index; boundary longitude and latitude: degrees; area: square kilometers.

## Domain

Every bounded A5 cell at resolutions 0–30. The abstract world cell `0` is deliberately rejected because it has no boundary.

## Approximations

The returned polyline samples curved spherical edges. Low-resolution cells receive more samples; the exact equal-area value comes from A5 itself.

## Worked example

- sourcePublisher: A5 project
- sourceTitle: A5 index encoding example
- sourceEdition: a5-js 0.10.1
- sourceLocator: cellToBoundary(0x63611a8000000000)
- independent: no (canonical a5-js cross-language reference; the tool remains experimental)
- inputs: cell 63611a8000000000
- outputs: 6-point closed boundary centered near 51.511842922° N, 0.159718399° W
- tolerance: 1e-9° per vertex
- verifiedBy: automated differential test
- verifiedOn: 2026-10-12

## Differential tests

- `core/crates/gp-indexing/tests/a5_parity.rs`: every vertex of 120 boundaries matches a5-js 0.10.1 within 1e-9°, including low-resolution densification, poles, and the antimeridian.

## Invariants

- The first and last boundary points are equal.
- The center decodes from the same cell and the area is positive.
