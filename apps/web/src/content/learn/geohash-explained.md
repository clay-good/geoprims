---
title: What is a geohash? Precision and edge cases
description: A geohash is a short string for a box on the map; each extra character makes the box smaller. How it is built, cell size by length, and where it breaks.
summary: How a geohash is built, how big a cell each length gives, why nearby points can have different prefixes, and when to reach for H3 or S2.
audience: Developers and GIS
published: 2026-09-23
tools:
  - indexing.geohash.encode
  - indexing.geohash.decode
  - indexing.geohash.neighbors
  - indexing.geohash.cover
  - indexing.convert.cross-index
sources:
  - title: Geohash tips and tricks (original description)
    issuer: Niemeyer, G., geohash.org, as archived by the Internet Archive
    edition: Archived March 5, 2008
    locator: Geohash format (precision, shared prefixes, trimming characters)
    url: https://web.archive.org/web/20080305223755/http://geohash.org/site/tips.html
  - title: "H3: Comparisons, Geohash"
    issuer: Uber Technologies and the H3 contributors
    edition: H3 4.x documentation
    locator: Area distortion and identifiers
    url: https://h3geo.org/docs/comparisons/geohash
  - title: S2 cell hierarchy
    issuer: S2 Geometry project
    edition: Developer guide, current
    locator: Cube faces, four children per cell, levels 0 to 30
    url: https://s2geometry.io/devguide/s2cell_hierarchy
---

A geohash is a short string of letters and digits that names a rectangle on the map. Each character narrows the rectangle, so a longer geohash means a smaller box. Downtown Pittsburgh at 9 characters is `dppn5fyxx`, a box about 4.77 m tall and 3.63 m wide. Trim characters from the end and you get the larger boxes that contain it.

## Why it matters

Geohashes are plain text, so they fit anywhere a string does: a database key, a URL, a log line, a cache key. Points that share a prefix are usually close together, which makes "find everything nearby" a cheap prefix query. That usefulness comes with sharp edges, and many geohash bugs come from them.

## How it is worked out

A geohash is built by halving the world again and again.

1. Start with longitude −180° to 180°. If the point is in the east half, write a 1. If it is in the west half, write a 0. Keep the half it is in.
2. Do the same for latitude, −90° to 90°.
3. Keep alternating, longitude first, one bit at a time.
4. Group the bits in fives and write each group as one character from a 32-character alphabet: `0123456789bcdefghjkmnpqrstuvwxyz`. The letters a, i, l, and o are left out, so they cannot be misread.

Each character carries 5 bits. Because the bits alternate and 5 is odd, one character adds 3 bits of longitude and 2 of latitude, and the next adds 2 and 3. That is why cells change shape between odd and even lengths.

Decoding runs the other way and gives a box, not a point. The [geohash decoder](/indexing/geohash/decode/) reports the center and the margin of error. For `dppn5fyxx` the center is 40.4461026°, −79.9822068°, give or take 0.0000215° in each direction.

## Cell size by length

Using the [geohash encoder](/indexing/geohash/encode/) on the same point (40.446111°, −79.982222°):

| Length | Geohash | Cell height | Cell width |
|---|---|---|---|
| 1 | d | 5,000,000 m | 4,620,000 m |
| 3 | dpp | 156,000 m | 120,000 m |
| 5 | dppn5 | 4,890 m | 3,720 m |
| 6 | dppn5f | 611 m | 930 m |
| 7 | dppn5fy | 153 m | 116 m |
| 8 | dppn5fyx | 19.1 m | 29.1 m |
| 9 | **dppn5fyxx** | **4.77 m** | **3.63 m** |
| 12 | dppn5fyxxjs9 | 0.0186 m | 0.0284 m |

Width depends on latitude, because meridians converge toward the poles. At 6 characters a cell is 611 m tall everywhere, but 1,220 m wide at the equator and 611 m wide at 60° N. The H3 documentation points out the same thing: geohash cells shrink in area away from the equator.

## Neighbors and edge effects

Each cell has eight neighbors. The [neighbors tool](/indexing/geohash/neighbors/) finds them by stepping one cell in each direction and encoding again. For `dppn5fyxx`, the north neighbor is `dppn5fyxz` and the east neighbor is `dppn5fyz8`. Those share most of their prefix, but not all: the east neighbor differs from the eighth character on.

At the big boundaries the prefix breaks down entirely. Four points 0.0001° north or south and east or west of 0°, 0° encode at 6 characters as:

| Point | Geohash |
|---|---|
| Just north and east of 0°, 0° | s00000 |
| Just north and west | ebpbpb |
| Just south and west | 7zzzzz |
| Just south and east | kpbpbp |

They share no characters at all, even though they are only a few tens of meters apart. The same thing happens, less dramatically, at every cell boundary at every length. So a prefix search for one cell will miss points just across its edge.

The fix is to search the cell **and its eight neighbors**. For an area, the [geohash cover tool](/indexing/geohash/cover/) lists every cell at a chosen length that touches a box or polygon.

## Rules of thumb

- **Each extra character divides the cell by 32.** Alternately 8 across and 4 down, then 4 across and 8 down.
- **Length 6 is about a neighborhood, 7 about a block, 9 about a parking space**, at mid-latitudes.
- **Shared prefix means close; different prefix does not mean far.** Always include neighbors.

## When to use H3 or S2 instead

Geohash is simple and needs no library. It is a good fit for coarse bucketing and string keys. Reach for something else when:

- **You compare areas or counts across latitudes.** Geohash cells shrink toward the poles. [H3](/learn/h3-resolution/) keeps cell areas much closer to equal.
- **You need even neighbors.** A hexagon's six neighbors are all about the same distance away. A rectangle's eight are not: the corner neighbors are farther.
- **You need a fast integer index or fine region coverings.** S2 projects a cube onto the sphere and splits each cell into four children, down to level 30, with cells about 1 cm across. Its ids are 64-bit integers.
- **You are drawing web maps.** [Map tiles and quadkeys](/learn/map-tiles-and-quadkeys/) are built for that.

To see one place in all of them at a matched size, use the [cross-index tool](/indexing/convert/cross-index/). For a cell size of about 150 m over Pittsburgh, it gives geohash `dppn5fy` (7 characters, about 133 m), H3 resolution 10 (about 123 m), and S2 level 16 (about 141 m), each measured as the square root of the cell's area.

## Common mistakes

- **Searching one cell only.** Include the eight neighbors, or use a cover.
- **Treating a geohash as a point.** It is a box. Store the original coordinates if you need them.
- **Assuming square cells.** Odd lengths are taller than they are wide at mid-latitudes, and even lengths are wider.
- **Comparing counts per cell across latitudes.** Cells near the poles are smaller.
- **Mixing lengths in one key column.** Pick one length, or store the full 12 characters and trim when you query.

## Where the numbers come from

The encoding is Gustavo Niemeyer's public-domain geohash, as described on the original geohash.org site. The [geohash encoder](/indexing/geohash/encode/) shows the cell size and bounds for any point and length, and the [decoder](/indexing/geohash/decode/) turns a geohash back into its box.
