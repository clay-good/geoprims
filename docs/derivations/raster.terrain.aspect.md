<!-- Derivation note (trust/correctness-program layer A). -->
# DEM aspect, the way the ground faces (`raster.terrain.aspect`)

## Method

Aspect is the direction the ground falls, read from the same Horn gradients as the slope. This operation returns the aspect in degrees clockwise from north and in words, from the same code as `raster.terrain.slope-aspect`. A window with no gradient has no aspect and is reported as flat.

## Equations

- dz/dx and dz/dy by Horn's method, as for slope
- Aspect = 90° − atan2(dz/dy, −dz/dx), brought into 0° to 360°
- Words: the nearest of the eight compass points, each covering 45°

## Symbols and units

Elevations in meters, cell sizes in meters; aspect in degrees clockwise from north, pointing downslope.

## Domain

One 3 x 3 window of finite elevations, north row first, and a positive cell size, in meters or in degrees with a latitude. Any other shape is refused with INVALID_INPUT.

## Approximations

None in the arithmetic. The window is all the operation sees: it describes the DEM, which smooths the ground and, for a surface model, includes buildings and canopy.

## Worked example

- sourcePublisher: the GDAL/OGR contributors (Open Source Geospatial Foundation)
- sourceTitle: gdaldem, GDAL's DEM tool
- sourceEdition: GDAL 3.13.3
- sourceLocator: `gdaldem aspect` (Horn) on the window written as an ASCII grid
- independent: yes
- inputs: a 10 m window falling to the north-east, rows 500, 498, 495 / 503, 500, 497 / 507, 504, 500
- outputs: aspect 45°, north-east, golden vector v002
- tolerance: 1e-9° against the Python transcription; gdaldem agrees within its 32-bit rounding, which on nearly flat ground is the larger bound
- verifiedBy: golden vectors v001 to v024, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_terrain_views.py`: 24 windows (five fixed, one flat, one exact plane, and 17 random slopes on 1 to 90 m cells), each value transcribed from the definition in 64-bit Python and each window also run through gdaldem; the script stops if they disagree beyond gdaldem's 32-bit rounding.
- `core/vectors/raster.terrain.aspect.jsonl`: those vectors and a refusal for a two-row window.
- `core/crates/gp-raster/tests/terrain_gdal.rs` `focused_terrain_tools_match_their_parents` requires `raster.terrain.slope-aspect`'s answer on 208 gdaldem fixture windows.

## Invariants

- `core/crates/gp-raster/tests/terrain_gdal.rs` `focused_terrain_invariants`: turning the window a quarter turn clockwise turns the aspect 90°.
- `core/crates/gp-raster/tests/terrain_gdal.rs` `focused_terrain_tools_match_their_parents`: the same aspect, and the same flat cells, as `raster.terrain.slope-aspect`, whose `terrain_matches_gdaldem` holds every gdaldem flat cell as flat.
- The tool takes no vertical exaggeration: scaling both gradients does not turn them.
