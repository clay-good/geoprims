<!-- Derivation note (trust/correctness-program layer A). -->
# Elevation statistics for a DEM grid (`raster.terrain.dem-stats`)

## Method

The operation reads a north-up grid of elevations, leaves out cells marked with the no-data value, and summarizes the rest: the mean, the lowest and highest values and the relief between them, the population standard deviation, and the median.

## Equations

- n = number of cells with data
- Mean = Σz / n
- Standard deviation = √(Σ(z − mean)² / n), the population form gdalinfo -stats reports
- Median = the middle sorted value, or the mean of the two middle values when n is even
- Relief = highest − lowest

## Symbols and units

Elevations z in meters; n a count of cells.

## Domain

At least two rows of two elevations, every row the same length, at most 300 by 300, with at least one cell holding data. Anything else is refused with INVALID_INPUT.

## Approximations

None: exact arithmetic on the values given. The statistics weight every cell alike, which on a geographic grid over-weights cells far from the equator.

## Worked example

- sourcePublisher: NumPy developers and the GDAL/OGR contributors
- sourceTitle: NumPy and gdalinfo
- sourceEdition: NumPy 2 (and GDAL 3.13.3 for the statistics)
- sourceLocator: NumPy's mean, std (ddof 0), median, min, and max; gdalinfo -stats on the grid written as an ASCII grid
- independent: yes
- inputs: a 4 by 5 site grid from 210.8 m to 215.2 m
- outputs: mean 213.235 m, standard deviation 1.2986 m, median 213.35 m, golden vector v001
- tolerance: 1e-9 m against NumPy; GDAL agrees within its float32 storage
- verifiedBy: golden vectors, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_dem_grid.py`: 24 grids from 2 by 2 to 12 by 12 (and 24 survey pairs for the volume), with no-data gaps and tied maxima, computed with NumPy rather than the core; each grid also read back by gdalinfo -stats, and the script stops if GDAL disagrees beyond its float32 storage.
- `core/vectors/raster.terrain.dem-stats.jsonl`: those vectors and refusals.

## Invariants

- Adding a constant to every cell adds it to the mean, median, lowest, and highest, and leaves the relief and standard deviation unchanged.
- The mean and median lie between the lowest and highest.
- Cells with the no-data value never change the answer.
