---
title: How to choose an H3 resolution
description: H3 has 16 resolutions, each cell about one seventh the area of the last. How to pick one from a target area or edge length, and where pentagons bite.
summary: Cell sizes at each H3 resolution, how to pick one for your data, and what pentagons and the hierarchy do to your results.
audience: Developers and GIS
published: 2026-09-23
tools:
  - indexing.h3.resolution-chooser
  - indexing.h3.lat-lng-to-cell
  - indexing.h3.grid-disk
  - indexing.h3.parent
  - indexing.h3.cell-info
sources:
  - title: "H3: Overview of the H3 geospatial indexing system"
    issuer: Uber Technologies and the H3 contributors
    edition: H3 4.x documentation
    locator: Base cells, pentagons, and aperture 7 resolutions
    url: https://h3geo.org/docs/core-library/overview
  - title: "H3: Tables of cell statistics across resolutions"
    issuer: Uber Technologies and the H3 contributors
    edition: H3 4.x documentation
    locator: Average cell area and edge length per resolution
    url: https://h3geo.org/docs/core-library/restable
  - title: "H3: Hierarchical grid functions"
    issuer: Uber Technologies and the H3 contributors
    edition: H3 4.x documentation
    locator: cellToParent, cellToChildren
    url: https://h3geo.org/docs/api/hierarchy
  - title: "H3: Grid traversal functions"
    issuer: Uber Technologies and the H3 contributors
    edition: H3 4.x documentation
    locator: gridDisk and pentagon distortion
    url: https://h3geo.org/docs/api/traversal
---

An H3 resolution is the size of the hexagons you divide the Earth into. H3 has 16 of them, numbered 0 (the coarsest, 122 cells for the whole planet) to 15 (cells under a square meter). Each step down makes cells about one seventh the area. To choose one, decide the cell size your question needs, then take the resolution whose average cell is closest: for cells of about 1 km², that is resolution 8.

## Why it matters

The resolution decides what your data can say. Cells that are too big blur a city block into a neighborhood. Cells that are too small leave most cells with one point or none, and your tables grow sevenfold with every step. Two datasets indexed at different resolutions do not join on cell id. Changing resolution later means re-indexing everything, so it is worth choosing with care up front.

## How it is worked out

H3 starts from an icosahedron, a 20-sided solid, set around the Earth. It lays a hexagon grid on each face and projects it onto the sphere. Resolution 0 has 122 base cells: 110 hexagons and 12 pentagons. Each finer resolution scales the cell edge by the square root of 7 and the cell area by one seventh.

The [H3 resolution chooser](/indexing/h3/resolution-chooser/) compares your target with the average cell at each resolution and returns the closest one, with the next coarser resolution for comparison. You can give either a target area or a target edge length.

## The resolution table

Average cell sizes from the resolution chooser, rounded:

| Resolution | Average area | Average edge | Cells worldwide |
|---|---|---|---|
| 0 | 4,357,449 km² | 1,281 km | 122 |
| 5 | 252.9 km² | 9.854 km | 2,016,842 |
| 6 | 36.13 km² | 3.725 km | 14,117,882 |
| 7 | 5.161 km² | 1.406 km | 98,825,162 |
| 8 | 0.7373 km² | 531.4 m | 691,776,122 |
| 9 | 0.1053 km² | 200.8 m | 4,842,432,842 |
| 10 | 15,050 m² | 75.86 m | 33,897,029,882 |
| 11 | 2,150 m² | 28.66 m | 237,279,209,162 |
| 12 | 307.1 m² | 10.83 m | 1,660,954,464,122 |
| 15 | 0.8953 m² | 0.5842 m | 569,707,381,193,162 |

## A worked example

You want cells of about 1 km² for a city-wide demand map.

| Step | Result |
|---|---|
| Target area | 1 km² |
| Closest resolution | **8**, averaging 0.7373 km² |
| Next coarser | 7, averaging 5.161 km² |

Resolution 8 is closest. If you think in distance instead, a target edge of 150 m gives resolution 9, with an average edge of 200.8 m.

Now index a point. Downtown Pittsburgh (40.446111°, −79.982222°) at resolution 9 is cell `892a8471487ffff`, about 0.1053 km². Its parent at resolution 5 is `852a8473fffffff`. It has 7 children at resolution 10.

## Cells are not all the same size

The table gives averages. Real cells vary with where they sit on the icosahedron. At resolution 9:

| Place | Cell area |
|---|---|
| Near 64° N, 10° E, close to a pentagon | 0.06531 km² |
| 0°, 0° | 0.07839 km² |
| Pittsburgh | 0.1053 km² |
| Sydney | 0.1267 km² |

The largest here is almost twice the smallest. If your analysis compares counts per cell across continents, divide by each cell's real area, which the [cell inspector](/indexing/h3/cell-info/) and [point-to-cell tool](/indexing/h3/lat-lng-to-cell/) report.

## Pentagons

A sphere cannot be tiled with hexagons alone. At every resolution H3 has exactly 12 pentagons, one centered on each vertex of the icosahedron. H3's orientation puts all 12 in the ocean, so most land data never meets one.

When you do meet one, the grid changes. A pentagon has five neighbors, not six. The pentagon cell `85080003fffffff`, off the coast of Norway, covers 127.786 km², about half the resolution 5 average. A ring of radius 1 around it holds 6 cells, not the usual 7, and the [grid disk tool](/indexing/h3/grid-disk/) warns you.

## Hierarchy and k-rings

**Parents and children.** Each hexagon has seven children at the next resolution. Hexagons do not split cleanly into seven smaller hexagons, so the H3 documentation says a parent only approximately contains its children. Some of a child's area can fall outside its parent's outline. Near cell edges, a point's parent cell can differ from the cell you get by indexing the point directly at the coarser resolution, so pick one method for roll-ups and keep to it. The [parent tool](/indexing/h3/parent/) gives the parent at any coarser resolution.

**Grid disks (k-rings).** All cells within k steps of a cell. Away from pentagons, a disk of radius k holds 3k(k + 1) + 1 cells: 7 for k = 1, 19 for k = 2, and 37 for k = 3. A disk is a quick "nearby" search. Its width in meters depends on the resolution, so choose k and resolution together.

## Rules of thumb

- **One step is seven times.** Moving one resolution finer multiplies the cell count by about 7.
- **Two steps is about 50 times.** Two resolutions finer is roughly 7 × 7 cells.
- **Pick the resolution for the question, not the data.** GPS points with a few meters of error still belong in large cells if the question is about neighborhoods.

## Common mistakes

- **Joining across resolutions.** Convert to a common resolution with the parent function first.
- **Treating the average as exact.** Cells vary by location, as the table above shows.
- **Assuming six neighbors everywhere.** Code that expects exactly 7 cells in a k = 1 disk breaks at pentagons.
- **Reading children as a perfect split.** A parent's outline and the union of its children do not match exactly.

## Where the numbers come from

The resolution table and cell values come from the H3 4.x library, as documented at h3geo.org. The [resolution chooser](/indexing/h3/resolution-chooser/) shows the full table with every answer. To compare H3 with other grids, see [geohash explained](/learn/geohash-explained/) and [map tiles and quadkeys](/learn/map-tiles-and-quadkeys/).
