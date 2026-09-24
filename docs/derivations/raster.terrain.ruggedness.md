<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Terrain ruggedness (`raster.terrain.ruggedness`)

## Method

Ruggedness measures how much the ground around a cell differs from the cell itself. The tool compares the center of a 3 x 3 window with its eight neighbors in three ways. The terrain ruggedness index (TRI) sums the differences; two versions share the name, Riley's root of the summed squared differences and the mean absolute difference that most GIS tools, including GDAL's Wilson algorithm, report, so both are given. The topographic position index (TPI) is the center minus the mean of its neighbors: positive on a ridge or summit, negative in a valley or hollow. Roughness is the range of the nine heights.

## Equations

- TRI (Riley 1999, per its erratum) = √(Σ (zᵢ − z₀)²) over the eight neighbors.
- TRI (mean, Wilson 2007) = (1/8) Σ |zᵢ − z₀|.
- TPI = z₀ − (1/8) Σ zᵢ.
- Roughness = max − min of the nine heights.

## Symbols and units

Inputs: `elevations` (three rows of three heights in meters, north row first) and the cell size (not used by these measures, which depend on heights only). Outputs: `tri`, `tri_mean`, `tpi`, and `roughness` in meters, and `position`, a phrase for the sign of TPI.

## Domain

One 3 x 3 window of finite heights.

## Approximations

None: the measures are exact functions of the nine heights.

## Worked example

- sourcePublisher: the GDAL/OGR contributors (Open Source Geospatial Foundation)
- sourceTitle: gdaldem, GDAL's DEM tool
- sourceEdition: GDAL 3.13.3
- sourceLocator: `gdaldem TRI -alg Riley`, `gdaldem TRI -alg Wilson`, `gdaldem TPI`, and `gdaldem roughness`, each written to an ASCII grid, on three projected DEMs written as ASCII grids
- independent: yes
- inputs: every interior cell of three 24 x 24 DEMs (5, 10, and 30 m cells; smooth hills with noise, one with a flat patch): 1,452 windows, 16 of them flat
- outputs: both TRIs, TPI, and roughness at each cell
- tolerance: 0.001 m
- verifiedBy: `core/crates/gp-raster/tests/terrain_gdal.rs` `terrain_matches_gdaldem`; golden vectors v008 to v021
- verifiedOn: 2026-09-24

gdaldem is the reference the spec names, and it offers both TRIs, so each is checked against its own algorithm. All four agree within 0.05 mm on every window; the difference is gdaldem's 32-bit arithmetic.

## Differential tests

- `tools/vectors/gen_terrain_gdal.py`: the gdaldem fixture (1,452 windows) and vectors v008 to v021, appended; v001 to v007 are transcriptions of the published formulas in Python (`tools/vectors/gen_raster.py`)
- `core/crates/gp-raster/tests/terrain_gdal.rs` `terrain_matches_gdaldem`: every window

## Invariants

- `core/crates/gp-raster/tests/terrain_gdal.rs` `terrain_invariants`: raising every height by 1,000 m changes none of the measures, and turning the ground upside down negates TPI and keeps both TRIs and roughness. `core/crates/gp-raster/tests/indices.rs` `ruggedness_places_the_cell_against_its_neighbours` holds the peak and pit cases.
