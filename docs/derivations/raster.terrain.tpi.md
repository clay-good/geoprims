<!-- Derivation note (trust/correctness-program layer A). -->
# Topographic position index (TPI) (`raster.terrain.tpi`)

## Method

The topographic position index compares a cell with the mean of its eight neighbors: positive for high ground such as a ridge, negative for low ground such as a valley. This operation returns it with a plain description, from the same code as `raster.terrain.ruggedness`.

## Equations

- TPI = z_0 − (Σ z_k) / 8 over the eight neighbors z_k of the center z_0

## Symbols and units

Elevations in meters; TPI in meters.

## Domain

One 3 x 3 window of finite elevations, north row first, and a positive cell size, in meters or in degrees with a latitude. Any other shape is refused with INVALID_INPUT.

## Approximations

None in the arithmetic. The window is all the operation sees: it describes the DEM, which smooths the ground and, for a surface model, includes buildings and canopy.

## Worked example

- sourcePublisher: the GDAL/OGR contributors (Open Source Geospatial Foundation)
- sourceTitle: gdaldem, GDAL's DEM tool
- sourceEdition: GDAL 3.13.3
- sourceLocator: `gdaldem TPI` on the window written as an ASCII grid
- independent: yes
- inputs: a ridge crest on a 30 m grid, rows 842.0, 845.5, 841.2 / 846.1, 851.3, 847.0 / 840.4, 844.8, 839.9
- outputs: TPI 7.9375 m, above its neighbours, golden vector v005
- tolerance: 1e-9 m against the Python transcription; gdaldem agrees within its 32-bit rounding
- verifiedBy: golden vectors v001 to v024, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_terrain_views.py`: 24 windows (five fixed, one flat, one exact plane, and 17 random slopes on 1 to 90 m cells), each value transcribed from the definition in 64-bit Python and each window also run through gdaldem; the script stops if they disagree beyond gdaldem's 32-bit rounding.
- `core/vectors/raster.terrain.tpi.jsonl`: those vectors and a refusal for a two-row window.
- `core/crates/gp-raster/tests/terrain_gdal.rs` `focused_terrain_tools_match_their_parents` requires `raster.terrain.ruggedness`'s answer on 208 gdaldem fixture windows.

## Invariants

- `core/crates/gp-raster/tests/terrain_gdal.rs` `focused_terrain_invariants`: inverting the ground negates the index, raising it changes nothing, and a plane through the window gives zero.
- `core/crates/gp-raster/tests/terrain_gdal.rs` `focused_terrain_tools_match_their_parents`: the same value as `raster.terrain.ruggedness` on 208 gdaldem fixture windows.
