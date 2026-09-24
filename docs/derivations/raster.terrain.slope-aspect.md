<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Slope, aspect, and hillshade (`raster.terrain.slope-aspect`)

## Method

A DEM gives the ground's height at the center of each cell. To say how steep the ground is at one cell, and which way it faces, the tool looks at the nine heights around it and estimates how fast the height changes toward the east and toward the north, using Horn's weighting: the row or column through the center counts twice as much as the ones beside it, which smooths out noise in a single height. The slope is the angle of that gradient; the aspect is the compass direction the ground faces downhill, clockwise from north; a window with no gradient is flat and has no aspect. Hillshade is the brightness of the cell lit by a sun at a given azimuth and altitude, scaled from 0 to 255, as GDAL computes it.

When the cell size is given in degrees, it is turned into meters with the WGS 84 radii of curvature at the latitude given, so that on a geographic grid the east-west cell, which shrinks with the cosine of the latitude, is not taken for the north-south one.

## Equations

- With the window a b c / d e f / g h i (a at the north-west): dz/dx = ((c + 2f + i) − (a + 2d + g)) / (8 Δx); dz/dy = ((g + 2h + i) − (a + 2b + c)) / (8 Δy).
- Slope = atan(z_factor · √((dz/dx)² + (dz/dy)²)); percent slope = 100 × the rise.
- Aspect = 90° − atan2(dz/dy, −dz/dx), taken into [0°, 360°); none when the gradient is zero.
- Hillshade = 255 · (cos Z cos S + sin Z sin S cos(A_sun − A)), clamped to 0 to 255, with Z the sun's zenith angle, S the slope, and A the aspect as angles in GDAL's convention.
- Degree cells: Δy = cell · M(φ) and Δx = cell · N(φ) cos φ, per radian of arc, with M and N the WGS 84 meridian and prime-vertical radii of curvature.

## Symbols and units

Inputs: `elevations` (three rows of three heights in meters, north row first), `cell_size` (meters) or `cell_degrees` with `lat`, `z_factor`, `sun_azimuth`, and `sun_altitude`. Outputs: `slope` (degrees), `slope_percent`, `aspect` (degrees clockwise from north, absent when flat), `aspect_text`, `hillshade` (0 to 255), and the cell sizes used.

## Domain

One 3 x 3 window of finite heights; a positive cell size, in meters or in degrees with a latitude.

## Approximations

None in the arithmetic, which is Horn's method as gdaldem computes it. The method itself smooths: a slope from a coarse DEM is gentler than the ground.

## Worked example

- sourcePublisher: the GDAL/OGR contributors (Open Source Geospatial Foundation)
- sourceTitle: gdaldem, GDAL's DEM tool
- sourceEdition: GDAL 3.13.3
- sourceLocator: `gdaldem slope`, `gdaldem aspect`, and `gdaldem hillshade` (Horn, sun at 315° and 45°), each written to an ASCII grid, on three projected DEMs written as ASCII grids
- independent: yes
- inputs: every interior cell of three 24 x 24 DEMs (5, 10, and 30 m cells; smooth hills with noise, one with a flat patch): 1,452 windows, 16 of them flat
- outputs: the slope, the aspect (no-data where flat), and the hillshade at each cell
- tolerance: slope to 0.001°; aspect to the spec's 0.01° from half a degree of slope up; hillshade to one step of 255
- verifiedBy: `core/crates/gp-raster/tests/terrain_gdal.rs` `terrain_matches_gdaldem`; golden vectors v008 to v021
- verifiedOn: 2026-09-24

gdaldem is the reference the spec names. Slope agrees within 0.00025° on every window. Aspect agrees within 0.0095° wherever the slope is at least half a degree. On nearly flat ground aspect is ill-conditioned, and gdaldem holds elevations as 32-bit floats, so its own rounding moves its aspect by up to about 0.02° at a slope of 0.1° on 30 m cells; there the test holds the tool to twice that bound, computed per window, while the tool works in 64-bit arithmetic and matches the formulas transcribed separately in Python on the windows of vectors v001 to v007, one of them to 1e-12 in `slope_and_aspect_follow_horn_as_gdaldem_does`. Every window gdaldem calls flat, the tool calls flat. Hillshade, an integer, differs by at most one step, from rounding.

## Differential tests

- `tools/vectors/gen_terrain_gdal.py`: the gdaldem fixture (1,452 windows) and vectors v008 to v021, appended; v001 to v007 are transcriptions of the published formulas in Python (`tools/vectors/gen_raster.py`)
- `core/crates/gp-raster/tests/terrain_gdal.rs` `terrain_matches_gdaldem`: every window

## Invariants

- `core/crates/gp-raster/tests/terrain_gdal.rs` `terrain_invariants`: on four windows, turning the window a quarter turn clockwise turns the aspect 90° and keeps the slope; raising every height by 1,000 m changes nothing; doubling the relief doubles tan(slope), and doubling the cell size halves it. `core/crates/gp-raster/tests/indices.rs` `slope_and_aspect_follow_horn_as_gdaldem_does` holds the spec's geographic-cell-size scenario (the east-west cell at 60° N half the north-south one).
