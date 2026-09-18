## Purpose

Encodes, decodes, and explains the square and quadtree spatial indexes (S2, geohash, quadkeys, web map tiles, and Plus Codes) so developers can move between them and see exactly which area each code covers.

## ADDED Requirements

### Requirement: S2 cells
The domain SHALL provide S2 tools with parity to S2 C++ for the covered API:
- **Conversion:** point ↔ cell ID at levels 0–30; cell ID ↔ token.
- **Inspection:** level, face, and Hilbert position.
- **Geometry:** cell vertices and center.
- **Hierarchy:** parent and children.
- **Neighbors:** edge neighbors and all neighbors at a level.
- **Measurement:** cell area (exact and average per level).
- **Region covering:** cap, rectangle, and polygon, with min/max level and max cells.

64-bit cell IDs SHALL be accepted and returned as strings.

#### Scenario: Token
- **WHEN** (40.446111°, -79.982222°) is converted to an S2 cell at level 15
- **THEN** the token is `8834f3dec`

#### Scenario: Region coverer parameters
- **WHEN** a polygon is covered with max cells = 8 and levels 10–16
- **THEN** at most 8 cells are returned, all between levels 10 and 16, and together they cover the polygon

### Requirement: Geohash
The domain SHALL encode geohashes at precision 1–12, decode them to a center and bounding box with error margins, compute the 8 neighbors (including across the antimeridian and at the poles, where neighbors may not exist), and cover a bounding box or polygon with geohashes at a precision.

#### Scenario: Encode
- **WHEN** (40.446111°, -79.982222°) is encoded at precision 9
- **THEN** the geohash is `dppn5fyxx`

#### Scenario: Invalid character
- **WHEN** a geohash containing `a` is decoded
- **THEN** the result is `INVALID_INPUT` stating that the geohash alphabet excludes a, i, l, and o

#### Scenario: Neighbors across the antimeridian
- **WHEN** the east neighbor of a cell touching +180° is requested
- **THEN** the neighbor wraps to the -180° side

### Requirement: Web map tiles (XYZ, TMS) and quadkeys
Tools SHALL convert lat/lon to XYZ tile (z 0–30), tile to lat/lon bounds, XYZ ↔ TMS (y_TMS = 2^z − 1 − y), tile ↔ quadkey, parent and children tiles, tiles covering a bounding box at a zoom (with a count limit), and ground resolution (m/px) at a latitude and zoom for 256 or 512 px tiles. Latitudes beyond ±85.05112878° SHALL be clamped with warning `WEB_MERCATOR_CLAMPED`.

#### Scenario: Tile and quadkey
- **WHEN** (40.446111°, -79.982222°) is converted at zoom 12
- **THEN** the XYZ tile is 12/1137/1544, the TMS y is 2551, and the quadkey is `032001112001`

#### Scenario: Ground resolution
- **WHEN** ground resolution is requested at that latitude, zoom 12, 256 px tiles
- **THEN** it is ≈ 29.08 m/px

#### Scenario: Tile y convention confusion
- **WHEN** a user enters `12/1137/2551` and selects "detect convention"
- **THEN** the tool shows both the XYZ and TMS interpretations, with their locations drawn

### Requirement: Plus Codes (Open Location Code)
Tools SHALL encode Plus Codes at code lengths 2–15, decode them to area bounds, shorten a code relative to a reference location, and recover a full code from a short code and a reference location. They SHALL match the official Open Location Code test data.

#### Scenario: Official test data
- **WHEN** the OLC project's encoding, decoding, shortening, and validity test files are run
- **THEN** every case passes

#### Scenario: Short code needs a reference
- **WHEN** a short code (e.g. `5FYX+XX`) is decoded without a reference location
- **THEN** the tool asks for a reference location and explains that short codes are ambiguous without one

### Requirement: Cross-index conversion
A tool SHALL convert a location or area among all supported indexes (H3, S2, geohash, quadkey, tile, Plus Code, Maidenhead, MGRS) at resolutions chosen to match a target cell size, reporting each system's actual cell size.

#### Scenario: Matched sizes
- **WHEN** the target cell size is about 150 m
- **THEN** the tool picks geohash precision 7 (≈ 153 m), H3 resolution 10 (≈ 0.015 km², about 123 m on a side as an equal-area square), and S2 level 16 (≈ 141 m average), and reports each actual size

### Requirement: Excluded proprietary systems
The catalog SHALL NOT implement what3words. Searching for it SHALL show an explanation (proprietary, patented, terms prohibit offline use) and suggest Plus Codes.

#### Scenario: what3words search
- **WHEN** a user searches for "what3words"
- **THEN** the palette shows the explanation entry and a link to Plus Codes tools
