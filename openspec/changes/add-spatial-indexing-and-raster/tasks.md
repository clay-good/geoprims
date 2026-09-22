## 1. H3

- [x] 1.1 Integrate `h3o` and implement indexing and inspection tools; verify the point-to-cell and parent/children scenarios (h3o 0.11: point to cell and a cell inspector with center, boundary, base cell, pentagon and Class III checks, area, and decimal form)
- [ ] 1.2 Implement traversal, hierarchy, compaction, edges, vertexes, and measurement; verify each against H3 C 4.5 fixtures (done so far: gridDisk, gridRing, gridPathCells and gridDistance, parent with child position, children with center child, compact (our own level-by-level version, because h3o's overflows on 32-bit wasm) and uncompact, directed edges with lengths, and vertexes, each checked against H3 C 4.4.1 vectors; pending: local IJ, great-circle distance, and cellsToMultiPolygon)
- [ ] 1.3 Implement polyfill with all containment modes, pre-fill size estimation, holes, antimeridian, and poles; verify the containment and estimate scenarios (done so far: center, full, and overlapping containment matching H3 C 4.4.1 exactly on 1,000 random polygons with holes and antimeridian crossings, ported from H3 C's polyfill tests on h3o's core API, since h3o's geo feature breaks reproducible builds; the estimate gate returns LIMIT_EXCEEDED before filling; GeoJSON Polygon and MultiPolygon input; pending: overlapping-bbox mode, pole-spanning polygons, and 1,000-cell pages over MCP)
- [x] 1.4 Implement pentagon handling and warnings; verify the pentagon-ring and path scenarios (done so far: PENTAGON_DISTORTION on disks, rings, children, and edges, and DEGENERATE_GEOMETRY naming H3's failure reason for paths; the pentagon-ring fixture draws the k = 1 disk on pentagon 85080003fffffff: six outlines, one origin and five around it rather than six, the distortion flagged, and the pentagon drawn with the ten boundary vertices H3 gives it at a Class III resolution and the five corners it has at a Class II one)
- [x] 1.5 Implement index input parsing with the 2⁵³ guard; verify the precision-loss scenario (hex with or without 0x in any case, or decimal strings; JSON numbers are refused with a hint)
- [x] 1.6 Implement the resolution chooser and table; verify the 1 km² scenario (the nearest resolution by area or edge length on a log scale, plus the full 16-row table)
- [x] 1.7 Run the H3 C differential suite (100,000 points × 16 resolutions plus API fixtures); add per-function C fallbacks for any gaps; verify full parity (100,000 random points × 16 resolutions against H3 C 4.4.1 (the newest available through h3-py): 0 index mismatches; centers within 4.5e-13° below 88° latitude and 1.8e-11° near the poles. A 250-point fixture runs in CI, and the full run is H3_DIFF=… cargo test -- --ignored)
- [ ] 1.8 Implement cell rendering with level of detail and compacted-set styling; verify the compacted-set fixture and a 1,000,000-cell render benchmark

## 2. A5, S2, geohash, tiles, Plus Codes

- [ ] 2.1 Integrate A5 (pinned version) as experimental; verify parity with a5-js and the experimental-label scenario
- [ ] 2.2 Port S2 cell math, neighbors, and RegionCoverer; verify the token and coverer scenarios and S2 C++ differential tests
- [x] 2.3 Implement geohash encode/decode/neighbors/cover; verify the encode, invalid-character, and antimeridian scenarios (done so far: encode, decode with error margins, and the 8 neighbors across the antimeridian and past the poles; the encode and invalid-character scenarios; and `indexing.geohash.cover` over a bounding box (across the antimeridian) or a polygon, cells that overlap or whose centers are inside, capped at 10,000)
- [x] 2.4 Implement tile, TMS, quadkey, bbox cover, and ground resolution; verify the tile, resolution, and convention scenarios (done so far: point to XYZ tile, TMS, and quadkey, tile or quadkey to bounds with a detect-convention reading, and ground resolution and scale for 256 or 512 px tiles, with WEB_MERCATOR_CLAMPED; and `indexing.tile.family` (parent and children, XYZ or TMS) and `indexing.tile.cover` (every tile over a box, across the antimeridian, capped at 10,000); all checked against an independent Python transcription)
- [x] 2.5 Integrate Open Location Code; verify all official OLC test files and the short-code scenario (all four official test files pass: encoding, decoding, validity, and shortening/recovery)
- [ ] 2.6 Implement cross-index conversion with matched sizes; verify the 150 m scenario
- [x] 2.7 Add the what3words explanation entry; verify the search scenario — searching for what3words, w3w, or a three-word address returns a note saying why it is not here (proprietary, patented, and its terms do not allow offline use) and pointing at the Plus Code tools, which the search results carry as `notes` and the palette and home search render as a row. A core test covers the wording, the suggested tools, that an unrelated query raises no note, and that a word merely containing a term does not raise one; a web test checks every tool it points at exists

## 3. Imagery indices

- [ ] 3.1 Implement the index catalog with citations and separate NDWI tools; verify the NDVI, NDWI, and zero-denominator scenarios
- [ ] 3.2 Implement sensor presets and scaling checks; verify the raw-DN and Sentinel-2 scenarios
- [ ] 3.3 Implement GeoTIFF/COG decoding and tiled per-pixel processing with no-data; verify the grid-mismatch and georeferencing scenarios
- [ ] 3.4 Implement the AST-based band-math evaluator with limits; verify the unknown-identifier scenario and fuzz the parser
- [ ] 3.5 Implement accessible index legends, histograms, and cursor readouts; verify the cursor scenario

## 4. Terrain

- [ ] 4.1 Build and host GLO-30 tiles with attribution and digests (from foundation task 5.6); verify the attribution and missing-coverage scenarios
- [ ] 4.2 Implement elevation lookup with height conversion; verify the ellipsoidal scenario
- [ ] 4.3 Implement profiles with sampling density and curvature option; verify the 10 km scenario
- [ ] 4.4 Implement Horn slope/aspect, hillshade, contours, TRI, TPI, and curvature with latitude-aware cell sizes; verify the geographic-cell and GDAL-agreement scenarios
- [ ] 4.5 Implement terrain line of sight with refraction and Fresnel clearance; verify the ridge and Fresnel scenarios
- [ ] 4.6 Implement the R2 viewshed in a worker with progress and cancel, plus GeoTIFF/GeoJSON export; verify the progress and 99%-agreement scenarios
- [ ] 4.7 Implement download-size confirmation and terrain offline packs; verify the 80 MB scenario

## 5. Catalog and docs

- [ ] 5.1 Register all indexing (52/92) and raster (30/40) operations and endpoints with aliases (hexbin, uber h3, s2 cell, slippy tile, open location code, vegetation index); verify catalog counts
- [ ] 5.2 Write docs per tool and the "Pick an H3 resolution" guide; verify the guide chain end to end
- [ ] 5.3 Promote tools meeting the stable bar; verify the verification report
