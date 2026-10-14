<!-- Derivation note (trust/correctness-program layer A). -->
# Highest point in a DEM grid (`raster.terrain.highest-point`)

## Method

The operation finds the largest elevation among the cells that hold data and reports the first cell that has it, reading north row first and west to east, with the count of cells that tie. With a cell size, it gives the cell's distance east and south of the north-west grid point.

## Equations

- Highest = max z over cells with data
- Row r and column c, counted from 1, of the first cell in reading order with z = highest
- East = (c − 1) × cell size; south = (r − 1) × cell size

## Symbols and units

Elevations and distances in meters; rows and columns as counts.

## Domain

As for the DEM statistics; the cell size, when given, must be more than zero. Anything else is refused with INVALID_INPUT.

## Approximations

The answer is a grid point; the ground's true high point can lie between points and stand higher.

## Worked example

- sourcePublisher: NumPy developers
- sourceTitle: NumPy
- sourceEdition: NumPy 2 (and GDAL 3.13.3 for the statistics)
- sourceLocator: NumPy argmax over the grid with no-data cells set to minus infinity, and the count of cells equal to the maximum
- independent: yes
- inputs: a knoll on a 2 m grid whose top value, 50.6 m, appears twice
- outputs: row 2, column 3, 4 m east and 2 m south, two cells tied, golden vector v002
- tolerance: 1e-9 m; rows, columns, and ties exactly
- verifiedBy: golden vectors, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_dem_grid.py`: 24 grids from 2 by 2 to 12 by 12 (and 24 survey pairs for the volume), with no-data gaps and tied maxima, computed with NumPy rather than the core.
- `core/vectors/raster.terrain.highest-point.jsonl`: those vectors and refusals.

## Invariants

- `core/crates/gp-raster/tests/demgrid.rs` `highest_point_invariants`: the highest value is at least every other value; raising one cell above all others moves the answer to that cell with one tie; and a cell set to the no-data value is never reported.
