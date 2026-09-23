<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Tile parent and children (`indexing.tile.family`)

## Method

A Web Mercator tile pyramid is a quadtree: each tile at zoom z splits into exactly four at zoom z + 1, and the arithmetic is integer halving and doubling with no geometry in it at all. The parent of (z, x, y) is (z − 1, ⌊x/2⌋, ⌊y/2⌋); the four children are (z + 1, 2x + i, 2y + j) for i and j in {0, 1}.

The part worth care is the y axis. XYZ — the slippy-map convention that OpenStreetMap and every web map use — counts y from the **north**. TMS counts it from the **south**, so TMS y = 2^z − 1 − XYZ y. The two agree at no zoom except where the grid is one tile tall, which makes an untested conversion look right at zoom 0 and wrong everywhere else. This tool takes and returns the convention you ask for, converting in and out around the same XYZ arithmetic.

A quadkey is the same tile written as a base-4 string, one digit per zoom level, each digit packing the low bit of x and y for that level. Giving a quadkey is giving a tile, and the children come back with theirs.

## Equations

- Parent: (z − 1, ⌊x/2⌋, ⌊y/2⌋), undefined at z = 0, which is reported as none.
- Children: (z + 1, 2x + i, 2y + j), i, j ∈ {0, 1}.
- TMS y = 2^z − 1 − XYZ y, both ways.
- Quadkey digit at level k = (bit k of x) + 2 · (bit k of y), most significant first.

## Symbols and units

`tile` is `z/x/y` or a quadkey; `convention` is `xyz` (the default) or `tms`. Out come `parent`, `parent_quadkey`, and the four `children`, each with its tile and quadkey.

## Domain

Zoom 0 to 20 or more; x and y within [0, 2^z). Zoom 0 has no parent, and the tool says none rather than returning a tile that does not exist.

## Approximations

None. This is integer arithmetic on a quadtree, exact at every zoom.

## Worked example

- sourcePublisher: OpenStreetMap contributors; Microsoft
- sourceTitle: Slippy map tilenames; Bing Maps Tile System
- sourceEdition: OSM wiki, current; Bing Maps Tile System (quadkey definition)
- sourceLocator: the slippy-map tile numbering and its TMS variant; the quadkey as a base-4 string, one digit per level
- independent: yes
- inputs: 23 tiles, including zoom 0, zoom 20, the four grid corners at zoom 7, three quadkeys, and three tiles given in TMS
- outputs: the parent and the four children, with quadkeys
- tolerance: exact
- verifiedBy: golden vectors v001 to v023, run by the core on every build
- verifiedOn: 2026-09-23

The expected tiles come from `tools/vectors/gen_cover.py` and `gen_cover_more.py`, which transcribe the slippy-map arithmetic in Python independently of the core. The cases worth having are the ones where a y-flip bug hides: (1/0/0) and (1/1/1), where the grid is two tiles tall, and (7/0/0) against (7/127/127), the opposite corners.

## Differential tests

- `tools/vectors/gen_cover.py` and `gen_cover_more.py`: all 23 vectors, from an independent Python transcription
- `core/crates/gp-indexing/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/indexing.tile.family.jsonl`: 23 vectors across zooms 0 to 20 and both conventions

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `tile_family_invariants`: every child's parent is the tile you started from, at every zoom tried, which is the quadtree property stated as a round trip; the four children are distinct and their quadkeys all extend the parent's by one digit; zoom 0 reports no parent rather than an invented one; the TMS and XYZ answers describe the same tiles, so converting a tile to TMS, asking for its family, and converting back gives the XYZ family; and a quadkey input gives the same answer as the z/x/y it encodes
