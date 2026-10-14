<!-- Derivation note (trust/correctness-program layer A). -->
# Volume change between two DEM grids (`raster.terrain.dem-difference-volume`)

## Method

The operation takes a before and an after grid on the same cells and forms the DEM of difference, after − before, cell by cell. Each grid point stands for a square cell of the cell size, as in a raster. Changes smaller than the minimum change count as none, which screens out survey noise (Wheaton and others 2010). Gains and losses are summed separately and together.

## Equations

- d = after − before at each cell with data in both grids; d = 0 when |d| < minimum change
- Net volume = Σd × cell²
- Gained = Σ d for d > 0, times cell²; lost = Σ −d for d < 0, times cell²
- Area changed = (cells with d ≠ 0) × cell²; mean change = Σd / cells compared

## Symbols and units

Elevations, the cell size, and the minimum change in meters; volumes in cubic meters; area in square meters.

## Domain

Two grids of the same shape, each at most 300 by 300; a cell size more than zero; a minimum change of zero or more; at least one cell with data in both. Anything else is refused with INVALID_INPUT.

## Approximations

Each point stands for a whole square cell, so edges and coarse grids are approximate. The true volume also carries each survey's vertical error.

## Worked example

- sourcePublisher: NumPy developers
- sourceTitle: NumPy
- sourceEdition: NumPy 2 (and GDAL 3.13.3 for the statistics)
- sourceLocator: NumPy on the two grids as arrays, with the minimum change applied by np.where
- independent: yes
- inputs: a stockpile surveyed on a 1 m grid before and after, with a 5 cm minimum change
- outputs: 5.40 m³ gained, 0.30 m³ lost, 5.10 m³ net over 11 changed cells, golden vector v001
- tolerance: 1e-9 m³ and 1e-9 m²
- verifiedBy: golden vectors, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_dem_grid.py`: 24 grids from 2 by 2 to 12 by 12 (and 24 survey pairs for the volume), with no-data gaps and tied maxima, computed with NumPy rather than the core.
- `core/vectors/raster.terrain.dem-difference-volume.jsonl`: those vectors and refusals.

## Invariants

- Swapping before and after swaps gained and lost and negates the net volume.
- Net volume = gained − lost.
- Raising the minimum change never increases the area changed.
