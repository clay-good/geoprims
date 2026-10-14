<!-- Derivation note (trust/correctness-program layer A). -->
# Hillshade value for one DEM cell (`raster.terrain.hillshade`)

## Method

A hillshade brightens ground that faces the sun and darkens ground that faces away. This operation computes the brightness of one cell the way gdaldem does, from Horn's slope and aspect and the sun's azimuth and altitude, with the same code as `raster.terrain.slope-aspect`.

## Equations

- cos i = cos Z cos S + sin Z sin S cos(A_sun − A), with Z = 90° − sun altitude, S the slope, A the aspect, and A_sun from the sun's azimuth, both in GDAL's angle convention
- Hillshade = 1 + 254 cos i, rounded; 1 where cos i ≤ 0 (gdaldem keeps 0 for no data)

## Symbols and units

Elevations and cell sizes in meters; sun azimuth and altitude in degrees (defaults 315° and 45°); hillshade a whole number from 1 to 255.

## Domain

One 3 x 3 window of finite elevations, north row first, and a positive cell size, in meters or in degrees with a latitude. Any other shape is refused with INVALID_INPUT.

## Approximations

None in the arithmetic. The window is all the operation sees: it describes the DEM, which smooths the ground and, for a surface model, includes buildings and canopy.

## Worked example

- sourcePublisher: the GDAL/OGR contributors (Open Source Geospatial Foundation)
- sourceTitle: gdaldem, GDAL's DEM tool
- sourceEdition: GDAL 3.13.3
- sourceLocator: `gdaldem hillshade -az 270 -alt 30` on the window written as an ASCII grid
- independent: yes
- inputs: a 5 m window lit from the west at 30°, rows 10, 12, 11 / 9, 13, 14 / 8.5, 11.5, 12.5
- outputs: hillshade 197, golden vector v003
- tolerance: exact
- verifiedBy: golden vectors v001 to v024, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_terrain_views.py`: 24 windows (five fixed, one flat, one exact plane, and 17 random slopes on 1 to 90 m cells), each value transcribed from the definition in 64-bit Python and each window also run through gdaldem; the script stops if they disagree beyond gdaldem's 32-bit rounding.
- `core/vectors/raster.terrain.hillshade.jsonl`: those vectors and a refusal for a two-row window.
- `core/crates/gp-raster/tests/terrain_gdal.rs` `focused_terrain_tools_match_their_parents` requires `raster.terrain.slope-aspect`'s answer on 208 gdaldem fixture windows.

## Invariants

- The value always lies between 1 and 255.
- Raising every elevation by the same amount leaves it unchanged.
- A flat cell gives 1 + 254 sin(sun altitude), rounded.
