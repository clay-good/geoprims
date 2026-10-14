<!-- Derivation note (trust/correctness-program layer A). -->
# DEM slope by Horn's method (`raster.terrain.slope`)

## Method

The slope of a DEM cell comes from Horn's eight-neighbor estimate of the elevation gradient across its 3 x 3 window. This operation returns only the slope, in degrees and percent, from the same code as `raster.terrain.slope-aspect`.

## Equations

- dz/dx = ((c + 2f + i) − (a + 2d + g)) / (8 Δx), dz/dy = ((g + 2h + i) − (a + 2b + c)) / (8 Δy), times the vertical exaggeration
- Slope = atan(√((dz/dx)² + (dz/dy)²)); percent = 100 √((dz/dx)² + (dz/dy)²)

## Symbols and units

a to i are the window's elevations in meters, row by row from the north-west corner; Δx and Δy the cell sizes in meters (from degrees with the WGS 84 radii at the latitude given). Slope in degrees.

## Domain

One 3 x 3 window of finite elevations, north row first, and a positive cell size, in meters or in degrees with a latitude. Any other shape is refused with INVALID_INPUT.

## Approximations

None in the arithmetic. The window is all the operation sees: it describes the DEM, which smooths the ground and, for a surface model, includes buildings and canopy.

## Worked example

- sourcePublisher: the GDAL/OGR contributors (Open Source Geospatial Foundation)
- sourceTitle: gdaldem, GDAL's DEM tool
- sourceEdition: GDAL 3.13.3
- sourceLocator: `gdaldem slope` (Horn) on the window written as an ASCII grid
- independent: yes
- inputs: a 10 m window on a hillside, rows 250.0, 252.5, 255.1 / 248.2, 251.0, 254.9 / 246.0, 249.3, 253.3
- outputs: slope 19.633° (35.67%), golden vector v001
- tolerance: 1e-9° against the Python transcription; gdaldem agrees within its 32-bit rounding
- verifiedBy: golden vectors v001 to v024, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_terrain_views.py`: 24 windows (five fixed, one flat, one exact plane, and 17 random slopes on 1 to 90 m cells), each value transcribed from the definition in 64-bit Python and each window also run through gdaldem; the script stops if they disagree beyond gdaldem's 32-bit rounding.
- `core/vectors/raster.terrain.slope.jsonl`: those vectors and a refusal for a two-row window.
- `core/crates/gp-raster/tests/terrain_gdal.rs` `focused_terrain_tools_match_their_parents` requires `raster.terrain.slope-aspect`'s answer on 208 gdaldem fixture windows.

## Invariants

- Raising every elevation by the same amount leaves the slope unchanged.
- Doubling the relief doubles tan(slope); doubling the cell size halves it.
- Turning the window a quarter turn keeps the slope.
