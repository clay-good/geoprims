---
title: Map tiles, zoom levels, and quadkeys explained
description: Web maps cut the world into square tiles named by zoom, x, and y. How XYZ, TMS, and Bing quadkeys number them, and how much ground a pixel covers.
summary: How web map tiles are numbered, why TMS flips the y axis, what a quadkey is, and the ground size of a pixel at each zoom.
audience: Developers and GIS
published: 2026-09-23
tools:
  - indexing.tile.from-point
  - indexing.tile.bounds
  - indexing.tile.ground-resolution
  - indexing.tile.cover
  - indexing.tile.family
sources:
  - title: Slippy map tilenames
    issuer: OpenStreetMap Wiki
    edition: Current wiki page
    locator: Lon./lat. to tile numbers, tile numbers to lon./lat., resolution and scale
    url: https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames
  - title: Bing Maps Tile System
    issuer: Microsoft
    edition: Online documentation
    locator: Latitude limit, tile coordinates and quadkeys, ground resolution
    url: https://learn.microsoft.com/en-us/bingmaps/articles/bing-maps-tile-system
  - title: Tile Map Service Specification
    issuer: OSGeo
    edition: Version 1.0, wiki page
    locator: TileMap origin (lower-left corner of tile 0/0)
    url: https://wiki.osgeo.org/wiki/Tile_Map_Service_Specification
---

A web map tile is a square image, usually 256 pixels across, that covers one piece of the world at one zoom level. At zoom 0 a single tile covers the whole map. Each zoom level splits every tile into four, so zoom z has 2^z tiles across and 2^z down. A tile is named by its zoom, column, and row, written z/x/y, or by a single string called a quadkey.

## Why it matters

Tile names show up everywhere: in tile server URLs, in offline map caches, in vector tile pipelines, and as a cheap spatial index. Three numbering schemes are in common use, and they disagree about which way the rows run. Mix them up and you fetch a tile from the wrong hemisphere. Knowing the ground size of a pixel at each zoom tells you which zoom to render, cache, or download.

## How it is worked out

### Web Mercator and the 85.05° limit

Nearly all web map tiles use the spherical Web Mercator projection (EPSG:3857). Mercator stretches toward the poles without end, so the square world map is cut off where it would stop being square: about 85.0511° north and south. Bing's documentation gives the limit as 85.05112878°. Points closer to the poles are not on the map. The [tile tool](/indexing/tile/from-point/) clamps them to the edge and says so.

### XYZ, TMS, and quadkeys

- **XYZ** (also called slippy map or Google numbering). x counts columns from the west edge at 180° W. y counts rows **down from the top**, the north edge. This is what OpenStreetMap and most web maps use.
- **TMS** (the OSGeo Tile Map Service convention). x is the same, but y counts rows **up from the bottom**, the south edge. The two are related by TMS y = 2^z − 1 − XYZ y. MBTiles files store rows this way.
- **Quadkey** (Bing Maps). One digit per zoom level, each 0 to 3, saying which quarter of the parent tile to go into. The quadkey's length is the zoom, and a tile's quadkey starts with its parent's. That makes quadkeys handy as database keys: all the tiles under one tile share a prefix.

### Ground resolution

The ground size of one pixel is:

**Ground resolution = cos(latitude) × 2π × 6,378,137 m ÷ (tile size × 2^zoom)**

It halves with every zoom level and shrinks toward the poles. Map scale follows from it at a screen resolution of 96 dots per inch.

## A worked example

Downtown Pittsburgh, 40.446111°, −79.982222°, at zoom 12:

| Form | Value |
|---|---|
| XYZ tile | **12/1137/1544** |
| TMS y | 2551 |
| Quadkey | 032001112001 |
| Ground resolution | 29.08 m per pixel |
| Map scale at 96 dpi | about 1 to 109,900 |

The [tile bounds tool](/indexing/tile/bounds/) shows that 12/1137/1544 spans latitude 40.3800284° to 40.4469471° and longitude −80.0683594° to −79.9804688°.

Now the classic mistake. Read the TMS row, 2551, as if it were XYZ, and tile 12/1137/2551 lands at latitude −40.4469471° to −40.3800284°: the same distance south of the equator, in the Pacific Ocean off Chile. The bounds tool's "detect" mode shows both readings side by side, so you can tell which convention a file uses.

## Ground resolution by zoom

At Pittsburgh's latitude, with 256-pixel tiles:

| Zoom | Meters per pixel | Scale at 96 dpi, about 1 to |
|---|---|---|
| 0 | 119,100 | 450,300,000 |
| 6 | 1,861 | 7,035,000 |
| 10 | 116.3 | 439,700 |
| 12 | 29.08 | 109,900 |
| 15 | 3.636 | 13,740 |
| 18 | 0.4545 | 1,718 |
| 20 | 0.1136 | 429.4 |

Latitude matters. At zoom 12 a pixel covers 38.22 m at the equator and 19.11 m at 60° N. With 512-pixel tiles, common for vector tiles, each pixel covers half as much ground: 14.54 m at zoom 12 over Pittsburgh.

## How many tiles?

Tile counts grow four times per zoom level. A small box over downtown Pittsburgh, 40.43° to 40.45° N and 80.02° to 79.98° W, takes 9 tiles at zoom 14 and 54 tiles at zoom 16. The [tile cover tool](/indexing/tile/cover/) lists them, which is useful for sizing an offline cache before you download it.

The [tile family tool](/indexing/tile/family/) goes up and down one level. The parent of 12/1137/1544 is 11/568/772, with quadkey 03200111200: the child's quadkey minus its last digit.

## Rules of thumb

- **Each zoom halves the pixel size** and quadruples the tile count.
- **Zoom 0 at the equator** is 156,500 m per pixel for 256-pixel tiles. Divide by 2 for each zoom level and multiply by cos(latitude).
- **The quadkey length is the zoom.** A 12-digit quadkey is a zoom 12 tile.

## Common mistakes

- **Mixing XYZ and TMS.** If every tile is in the wrong hemisphere or flipped top to bottom, the y axis is upside down.
- **Forgetting the latitude.** Meters per pixel at the equator overstates it everywhere else.
- **Assuming 256-pixel tiles.** Many vector tile styles use 512, which shifts every zoom level by one.
- **Expecting polar coverage.** Web Mercator tiles stop at about 85.05°. Use another projection for polar work.
- **Using tiles for area analysis.** Tiles far from the equator cover much less ground than tiles near it. For equal-area cells, see [how to choose an H3 resolution](/learn/h3-resolution/).

## Where the numbers come from

Tile math follows the OpenStreetMap wiki's slippy map page and Microsoft's Bing Maps Tile System article. The TMS row order comes from the OSGeo Tile Map Service specification. The [tile tool](/indexing/tile/from-point/) gives all three names for any point, and the [ground resolution tool](/indexing/tile/ground-resolution/) gives meters per pixel and scale at any zoom and latitude. For string-based cells, see [geohash explained](/learn/geohash-explained/).
