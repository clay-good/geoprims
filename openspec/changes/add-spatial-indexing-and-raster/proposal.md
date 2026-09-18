## Why

Developers and analysts work with spatial indexes (H3, S2, geohash, quadkeys, map tiles, Plus Codes) every day. Their tooling is scattered across single-purpose viewers and library REPLs, and indexes are easy to misread: which resolution, which containment rule, TMS or XYZ row order? Remote-sensing and terrain users need band math and terrain analysis without installing GDAL or uploading imagery.

geoprims offers every major index with instant decode-and-draw, correct polyfill semantics, and client-side raster math on the user's own files or self-hosted open terrain.

Depends on: `establish-platform-foundation`, `add-geodesy-suite`, `add-navigation-and-geometry`.

## What Changes

Adds the `indexing` domain (about 52 operations and 92 endpoints) and the `raster` domain (about 30 operations and 40 endpoints). Inventory is in `design.md`.

- **Hexagonal grids:** H3 v4 (full core API: indexing, inspection, traversal, hierarchy, regions with all containment modes, directed edges, vertexes, measurement, validation) and A5 pentagonal cells (experimental while pre-1.0).
- **Hierarchical cells and tiles:** S2 cells (levels 0–30, tokens, region covering), geohash (encode, decode, neighbors, bbox cover), Bing quadkeys, XYZ and TMS tiles (conversions, bounds, ground resolution, tile ranges), and Plus Codes (Open Location Code: encode, decode, shorten, recover).
- **Imagery indices:** NDVI, NDWI (both definitions, labeled), MNDWI, EVI, EVI2, SAVI, NDBI, NBR and dNBR, and a safe band-math expression tool. Includes Sentinel-2 and Landsat 8/9 band and scaling presets, on single values or local GeoTIFFs.
- **Terrain analysis:** elevation lookup, elevation profile, slope, aspect, hillshade, contours, terrain line of sight with Earth curvature and refraction, and viewshed, on self-hosted Copernicus GLO-30 tiles or user-supplied DEMs.

## Capabilities

### New Capabilities

- `indexing/hexagonal-grids`: H3 and A5 discrete global grid operations.
- `indexing/hierarchical-cells`: S2, geohash, quadkey, map tiles, and Plus Codes.
- `raster/imagery-indices`: Spectral indices and band math on values and local rasters.
- `raster/terrain-analysis`: DEM-based elevation, profile, surface derivatives, line of sight, and viewshed.

### Modified Capabilities

None.

## Non-goals

- what3words (proprietary, patented, and its terms forbid offline implementation).
- Satellite imagery search or download (no server, no third-party requests). Users bring their own files.
- Hydrology (flow direction, watersheds) and full raster GIS in v1.
- Real-time elevation APIs.

## Impact

- `core/gp-indexing` and `core/gp-raster` crates.
- Assets: `dem-glo30` (self-hosted, re-tiled, attribution required), `h3-res0`, `s2-faces`.
- Differential references: H3 C 4.5 and h3-js 4.5, S2 C++ and s2js, the official Open Location Code implementations, a5-js, and GDAL (`gdaldem`) for terrain derivatives.
