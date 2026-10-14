<!-- Derivation note (trust/correctness-program layer A). -->
# Terrain ruggedness index (TRI) (`raster.terrain.roughness-tri`)

## Method

The terrain ruggedness index measures how much a cell's eight neighbors differ from it. Two indices share the name, and this operation returns both, from the same code as `raster.terrain.ruggedness`: Riley's root of summed squares and the mean absolute difference.

## Equations

- Riley: TRI = √(Σ (z_k − z_0)²) over the eight neighbors z_k of the center z_0
- Mean absolute (Wilson): Σ |z_k − z_0| / 8

## Symbols and units

Elevations in meters; both indices in meters.

## Domain

One 3 x 3 window of finite elevations, north row first, and a positive cell size, in meters or in degrees with a latitude. Any other shape is refused with INVALID_INPUT.

## Approximations

None in the arithmetic. The window is all the operation sees: it describes the DEM, which smooths the ground and, for a surface model, includes buildings and canopy.

## Worked example

- sourcePublisher: the GDAL/OGR contributors (Open Source Geospatial Foundation)
- sourceTitle: gdaldem, GDAL's DEM tool
- sourceEdition: GDAL 3.13.3
- sourceLocator: `gdaldem TRI -alg Riley` and `-alg Wilson` on the window written as an ASCII grid
- independent: yes
- inputs: broken ground on a 30 m grid, rows 1210.4, 1225.9, 1218.2 / 1204.7, 1220.0, 1231.6 / 1199.3, 1212.8, 1226.1
- outputs: Riley 31.881 m and mean absolute 9.775 m, golden vector v004
- tolerance: 1e-9 m against the Python transcription; gdaldem agrees within its 32-bit rounding
- verifiedBy: golden vectors v001 to v024, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_terrain_views.py`: 24 windows (five fixed, one flat, one exact plane, and 17 random slopes on 1 to 90 m cells), each value transcribed from the definition in 64-bit Python and each window also run through gdaldem; the script stops if they disagree beyond gdaldem's 32-bit rounding.
- `core/vectors/raster.terrain.roughness-tri.jsonl`: those vectors and a refusal for a two-row window.
- `core/crates/gp-raster/tests/terrain_gdal.rs` `focused_terrain_tools_match_their_parents` requires `raster.terrain.ruggedness`'s answer on 208 gdaldem fixture windows.

## Invariants

- Raising every elevation by the same amount leaves both unchanged.
- Turning the ground upside down leaves both unchanged.
- Neither is ever negative.
