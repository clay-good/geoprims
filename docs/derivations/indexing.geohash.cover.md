<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Geohashes covering an area (`indexing.geohash.cover`)

## Method

A geohash is a rectangle in latitude and longitude, named by a base-32 string. Each character adds five bits, alternating longitude and latitude starting with longitude, so a precision-p cell is 360° / 2^⌈5p/2⌉ wide and 180° / 2^⌊5p/2⌋ tall. Odd precisions get an extra longitude bit, which is why geohash cells alternate between wide and tall as precision grows rather than shrinking evenly.

Covering an area is then enumerating the grid rectangles that meet it. The tool offers two readings, and which one you want depends on what the cover is for:

- **overlap** (the default) keeps every cell that touches the area, so the union of the cells contains the area. This is what you want for a query that must not miss anything.
- **center** keeps only cells whose centre falls inside, so the cells are contained by the area, more or less. This is what you want when each cell will be treated as wholly inside.

The two differ most on thin shapes, and a polygon smaller than a single cell covers nothing at all in centre mode — which is the correct answer and the reason overlap mode exists.

Polygon edges are treated as straight in latitude and longitude, the same space the cells live in, so the test is a planar clip and not a geodesic one. That is the right choice here: a geohash cell is itself a lat/lon rectangle, so anything else would be comparing a shape against a grid defined in different terms.

## Equations

- Cell size at precision p: 360° / 2^⌈5p/2⌉ wide, 180° / 2^⌊5p/2⌋ tall.
- Cell indices: column = ⌊(lon + 180) / width⌋, row = ⌊(lat + 90) / height⌋.
- Overlap mode keeps a cell when it intersects the box, or when the polygon clipped to the cell has area, or when a polygon vertex lies in it.
- Centre mode keeps a cell when its centre is inside the box, or inside the polygon by the crossing-number test.

## Symbols and units

`precision` is 1 to 12; give a `bbox` (`south, west, north, east`) or a `polygon`; `mode` is `overlap` (the default) or `center`. Out come `count` and the `geohashes`.

## Domain

Any box or polygon, any precision the cap allows. A box whose west is greater than its east crosses the antimeridian and the columns wrap.

## Approximations

None in the grid arithmetic, which is exact binary subdivision. The approximation is the planar polygon test, stated above and deliberate: over a cell a few kilometres across, a geodesic edge and a straight lat/lon edge differ by far less than a cell.

## Worked example

- sourcePublisher: Gustavo Niemeyer (geohash definition); OpenStreetMap contributors
- sourceTitle: The geohash algorithm and its base-32 alphabet
- sourceEdition: the public definition as implemented from scratch in `tools/vectors/gen_cover.py`
- sourceLocator: five bits per character, alternating longitude and latitude, longitude first; alphabet `0123456789bcdefghjkmnpqrstuvwxyz`
- independent: yes
- inputs: 22 areas, including precisions 1 through 8, both modes, a box crossing the antimeridian, the south-west corner of the world, a polygon smaller than one cell, and a thin sliver where the two modes disagree most
- outputs: the cell count and the first and last geohash
- tolerance: exact
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

The reference encodes geohashes from scratch — the bit interleaving and the base-32 alphabet, not a library — and does the overlap by Sutherland-Hodgman clipping and the centre test by crossing number. So the comparison is against the definition rather than against another implementation that might share a misreading of it.

## Differential tests

- `tools/vectors/gen_cover.py` and `gen_cover_more.py`: all 22 vectors, from a from-scratch encoder and an independent clip
- `core/crates/gp-indexing/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/indexing.geohash.cover.jsonl`: 22 vectors over precisions 1 to 8 and both modes

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `geohash_cover_invariants`: centre mode never returns more cells than overlap mode, and every cell it returns is one overlap mode returns too, so the two modes nest rather than merely differing; every geohash returned has exactly the requested precision; each one decodes back inside the area it was asked for, checked through `indexing.geohash.decode`; a polygon smaller than one cell covers nothing in centre mode and something in overlap mode, which is the pair of answers that shows what the modes mean; and one more character of precision returns more cells for the same area, since each cell splits into 32
