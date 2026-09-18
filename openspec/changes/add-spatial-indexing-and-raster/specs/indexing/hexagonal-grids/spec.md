## Purpose

Exposes the full H3 hexagonal grid system, plus the emerging A5 equal-area pentagonal grid, with exact parity with the reference implementations. Every cell and set of cells is drawn instantly.

## ADDED Requirements

### Requirement: H3 v4 API parity
The domain SHALL provide tools equivalent to the H3 v4.x core API, named in v4 vocabulary:
- **Indexing:** `latLngToCell`, `cellToLatLng`, `cellToBoundary`.
- **Inspection:** resolution, base cell, pentagon check, Class III check, validity.
- **Traversal:** `gridDisk`, `gridRing`, `gridPathCells`, `gridDistance`, local IJ coordinates.
- **Hierarchy:** parent, children, center child, child position, compact, uncompact.
- **Regions:** `polygonToCells` with all containment modes (center, full, overlapping, overlapping-bbox), `cellsToMultiPolygon`.
- **Edges and vertexes:** directed edges and vertexes.
- **Measurement:** cell area, edge length, great-circle distance.
- **Resolution tables:** cell counts and average areas.

Every output SHALL match H3 C 4.5 for the same inputs, with indexes identical and coordinates within 1e-12°.

#### Scenario: Point to cell
- **WHEN** (40.446111°, -79.982222°) is indexed at resolution 9
- **THEN** the cell is `892a8471487ffff`, and its center is ≈ (40.444866°, -79.981847°)

#### Scenario: Parent and children
- **WHEN** the parent at resolution 5 and the children at resolution 10 of `892a8471487ffff` are requested
- **THEN** the parent is `852a8473fffffff` and there are 7 children

#### Scenario: Differential parity
- **WHEN** 100,000 random points across all 16 resolutions are indexed
- **THEN** every index equals H3 C 4.5's output

### Requirement: Pentagon and distortion handling
Tools SHALL handle the 12 pentagons at each resolution correctly: k-rings around pentagons have fewer cells, some IJ and path operations fail near pentagons, and results SHALL flag pentagon involvement (`PENTAGON_DISTORTION`) rather than erroring silently.

#### Scenario: Ring around a pentagon
- **WHEN** `gridDisk` with k = 1 is requested for pentagon `85080003fffffff`
- **THEN** 6 cells are returned (not 7) with `PENTAGON_DISTORTION`

#### Scenario: Path across a pentagon
- **WHEN** `gridPathCells` cannot be computed because the line crosses pentagon distortion
- **THEN** the tool returns an error with code `DEGENERATE_GEOMETRY` naming the H3 failure reason

### Requirement: Polyfill with explicit containment and limits
`polygonToCells` SHALL require an explicit containment mode (default "center", with the mode echoed). It SHALL estimate output size before computing and return `LIMIT_EXCEEDED` above the declared limit (default 5,000,000 cells in the web app, and a paginated 1,000 per call via MCP). It SHALL correctly handle polygons with holes, across the antimeridian, and around poles.

#### Scenario: Containment mode changes counts
- **WHEN** the same polygon is filled with "center" and with "overlapping"
- **THEN** the overlapping result is a superset of the center result, and both counts are shown

#### Scenario: Estimate before fill
- **WHEN** a continent-scale polygon is requested at resolution 12
- **THEN** the tool returns `LIMIT_EXCEEDED` with the estimate within 50 ms, suggesting a coarser resolution or compaction

### Requirement: Index input tolerance
H3 inputs SHALL accept hexadecimal strings (with or without a `0x` prefix, any case) and 64-bit integers given as decimal strings. They SHALL reject JavaScript numbers above 2⁵³, with a hint to pass a string, and SHALL validate cells, edges, and vertexes by mode.

#### Scenario: Precision loss guard
- **WHEN** an H3 index is supplied as a JSON number
- **THEN** the tool returns `INVALID_INPUT` explaining that 64-bit indexes must be strings

### Requirement: Resolution chooser
A tool SHALL recommend H3 resolutions for a target cell area or edge length and show the resolution table (0–15: average hexagon area, average edge length, cell counts, 122 base cells, 12 pentagons each).

#### Scenario: Target 1 km² cells
- **WHEN** the target area is 1 km²
- **THEN** the tool recommends resolution 8 (≈ 0.74 km² average) and shows resolution 7 (≈ 5.16 km²) as the next coarser

### Requirement: A5 pentagonal grid (experimental)
The domain SHALL provide A5 cell indexing, boundary, parent/children, and cell area. A5 is an equal-area pentagonal DGGS with 31 resolutions. The tools SHALL be labeled experimental while the reference implementation is pre-1.0, and SHALL match a5-js outputs for the pinned version.

#### Scenario: Experimental label
- **WHEN** a user opens an A5 tool
- **THEN** the page shows the EXPERIMENTAL badge and the pinned reference version

### Requirement: Cell visualization
Tools SHALL draw cells, rings, polyfills (with holes), compacted sets (mixed resolutions, visually distinguishable), and edges on the map and globe. They SHALL support up to 1,000,000 drawn cells through level-of-detail aggregation.

#### Scenario: Compacted set rendering
- **WHEN** a compacted set with three resolutions is drawn
- **THEN** cells of each resolution are distinguishable by outline weight and label, not color alone
