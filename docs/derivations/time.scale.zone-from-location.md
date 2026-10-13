<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Time zone from coordinates (`time.scale.zone-from-location`)

## Method

This is a point-in-polygon lookup over timezone-boundary-builder's comprehensive 2026e dataset. The optional boundary pack is loaded through the same checked asset path as other large reference data, and the exact release is echoed in `meta.assets`.

The build pipeline verifies the upstream ZIP's pinned SHA-256 digest, reads its GeoJSON, sorts zones by IANA identifier, rounds each source coordinate to a microdegree, and delta-varint encodes every vertex. It does not simplify the geometry. At run time the core first rejects polygons by their bounding boxes, then applies the even-odd rule to the outer ring and holes. A point on a segment counts as contained. If two mapped zones contain a border point, the alphabetically first IANA identifier wins so the answer is deterministic.

## Equations

For a horizontal ray from point `P`, a ring toggles inside/outside at each edge whose latitude range straddles `P` and whose intersection lies east of `P`. The implementation compares integer cross-products instead of dividing, so vertical edges and vertices do not introduce floating-point branch changes. Polygon containment is the exclusive-or of its rings, which subtracts holes.

## Symbols and units

`lat` and `lon` are WGS 84 coordinates in degrees. The index stores longitude and latitude as signed integers in millionths of a degree. The output `zone` is an IANA identifier such as `America/Denver`.

## Domain

Mapped land boundaries in timezone-boundary-builder 2026e. Longitude is normalized to `[-180°, 180°)`, and the antimeridian is also tested as `+180°` when needed.

## Approximations

Each source ordinate is rounded by at most `0.0000005°`, under 0.06 m north-south and no larger east-west. No vertex is otherwise simplified or omitted. The result still inherits OpenStreetMap's boundary completeness and accuracy. The comprehensive distribution does not assign open-ocean nautical offsets, and the tool returns `NO_SOLUTION` there instead of guessing from longitude.

## Worked example

- sourcePublisher: timezone-boundary-builder contributors and OpenStreetMap contributors
- sourceTitle: timezone-boundary-builder comprehensive time zone boundary set
- sourceEdition: 2026e
- sourceLocator: `combined.json`, feature `America/Denver`
- independent: yes
- inputs: Denver at 39.7392° N, 104.9903° W, 19 other cities across six continents, and an open-Pacific point
- outputs: `America/Denver`; the other cities' published IANA identifiers; `NO_SOLUTION` over open ocean
- tolerance: exact identifier after source coordinates are rounded to a microdegree
- verifiedBy: golden vectors v001 to v021 and the public no-asset scenario test
- verifiedOn: 2026-10-12

## Differential tests

- `tools/data/tz-boundaries.py`: independently reads the pinned upstream GeoJSON ZIP and produces the compact index without simplification
- `tools/data/practitioner-assets.test.mjs`: checks the source archive, binary signature, ODbL license, registry digest, and that every boundary identifier exists in the bundled tzdb
- `core/vectors/time.scale.zone-from-location.jsonl`: 20 city lookups across six continents plus the open-ocean refusal

## Invariants

- `core/crates/gp-time/src/boundary.rs` unit tests pin integer ray crossings, exact boundary detection, and malformed-header refusal
- `core/crates/gp-time/tests/time.rs` `zone_from_location_scenarios` proves the tool returns `ASSET_UNAVAILABLE` before the optional data is supplied, names its 28.3 MB download and manual zone/offset fallback, then returns the zone and exact asset version after the same bytes are supplied
