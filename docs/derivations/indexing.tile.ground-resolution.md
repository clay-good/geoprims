<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Map tile ground resolution (`indexing.tile.ground-resolution`)

## Method

Web Mercator draws the equator (2π × 6,378,137 m) across 2^z tiles of 256 or 512 pixels. The Mercator scale factor at latitude φ is sec φ, so one pixel covers the equatorial resolution times cos φ on the ground. The map scale at the screen's standard pixel is the resolution divided by the pixel's physical size.

## Equations

- Ground resolution: r = cos φ × 2π × 6,378,137 / (s × 2^z) meters per pixel.
- Map scale: 1 : r / p, with p the screen pixel size (0.264583 mm, 96 dpi).

## Symbols and units

φ latitude (degrees), z zoom level (0–30), s tile size in pixels (256 or 512), r meters per pixel, p pixel size in meters.

## Domain

Latitude −85.0511° to 85.0511° (Web Mercator's limit; latitudes beyond it are clamped with a WEB_MERCATOR_CLAMPED warning), zoom 0 to 30, tile size 256 or 512 pixels.

## Approximations

Web Mercator is a sphere of radius 6,378,137 m, so r is the distance on that sphere, not on the WGS 84 ellipsoid: it differs from a true ground distance by up to 0.7%. This is the convention every web map uses.

## Worked example

- sourcePublisher: Microsoft
- sourceTitle: Bing Maps Tile System
- sourceEdition: article of 2018-02-28
- sourceLocator: ground-resolution table, level 1 at the equator: 78,271.5170 m per pixel (levels 3 to 23 also pinned)
- independent: yes
- inputs: lat 0, zoom 1, tile size 256
- outputs: 78,271.5170 m per pixel
- tolerance: 5e-5 m (the table prints four decimals)
- verifiedBy: golden vector v022, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-indexing/tests/dev_parity.rs`: `ground_resolution_matches_mercantile_tile_widths` checks 1,000 random tiles (zoom 0 to 24) against the width of mercantile 1.2.1's tile bounds over 256 pixels, within 1e-9 relative
- `core/vectors/indexing.tile.ground-resolution.jsonl`: 30 vectors, nine of them from the Bing table

## Invariants

- `core/crates/gp-indexing/tests/indexing.rs` `ground_resolution_invariants`: each zoom level halves the resolution, it scales with cos φ, 512-pixel tiles halve it again, and the scale is a fixed multiple of it
