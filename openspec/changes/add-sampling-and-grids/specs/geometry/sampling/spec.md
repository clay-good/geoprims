## Purpose

Generates the grids, sample points, and transects a field plan needs, inside a polygon and in a stated projected plane, with every random layout reproducible from its seed on both surfaces.

## ADDED Requirements

### Requirement: Seeded, deterministic randomness
Sampling tools SHALL draw randomness only from xoshiro256** seeded by splitmix64 from a user-supplied integer seed (0 to 2⁶⁴ − 1, default 1), with doubles formed as `(x >> 11) × 2⁻⁵³`. The same seed and inputs SHALL give byte-identical points on the website and the MCP server. The seed SHALL be echoed in the result and pinned in permalinks.

#### Scenario: Reference stream
- **WHEN** the generator is seeded with 42
- **THEN** its first 10 outputs equal the authors' reference implementation's, pinned as a vector

#### Scenario: Same seed, both surfaces
- **WHEN** 500 random points are requested for a polygon with seed 7 on the web and through `geoprims_run`
- **THEN** the two point lists are byte-identical

### Requirement: Grid generation
The fishnet tool SHALL build rectangular cells (as polygons, cell-center points, or lines) of a given width and height over a polygon or bounding box in a chosen projected CRS (the polygon's UTM zone by default), from an origin (default the lower-left of the bounding box) with an optional rotation, and SHALL clip cells to the polygon when asked, reporting each partial cell's inside fraction. The hex-grid tool SHALL do the same with flat-top hexagons of width w, horizontal spacing w, vertical spacing 0.866 w, rows offset by w/2. Results SHALL echo the CRS, origin, rotation, cell count, and the maximum scale-factor distortion over the area.

#### Scenario: QGIS parity
- **WHEN** a 100 m fishnet is built over a 1 km × 1 km square in its UTM zone with the default origin
- **THEN** 100 cells are returned whose corners match QGIS "Create grid" output within 1 mm

#### Scenario: Area too large
- **WHEN** the polygon's extent exceeds 500 km
- **THEN** the result is `LIMIT_EXCEEDED` explaining that a single planar grid would distort beyond the stated bound

### Requirement: Random points in a polygon
The random-points tool SHALL place N points uniformly inside a polygon (holes excluded) by triangulating it, choosing a triangle with probability proportional to area, and placing the point by Osada et al. 2002 §4.2, using exactly two draws per point. It SHALL return lat/lon rows, the area each point represents (polygon area ÷ N, m²), the seed, and the design citation.

#### Scenario: All inside
- **WHEN** 10,000 points are placed in an L-shaped polygon with a hole
- **THEN** every point lies inside the polygon and outside the hole

#### Scenario: Uniform by area
- **WHEN** 100,000 points are placed in a polygon split into two equal-area halves
- **THEN** each half receives between 49,000 and 51,000 points

### Requirement: Systematic and stratified designs
The systematic tool SHALL place points on a square or triangular grid of spacing d with a random start offset from the seed (or none), per EPA QA/G-5S chapter 5. The stratified tool SHALL use fishnet or hex cells clipped to the polygon as strata and place k random points in each, allocating strata in row-major order so the seed stream is reproducible. Both SHALL name the design and its source in the result.

#### Scenario: Triangular grid spacing
- **WHEN** a triangular systematic design with d = 50 m is generated
- **THEN** each interior point's six nearest neighbors are 50 m away (±1 mm); points next to the boundary have fewer

#### Scenario: Empty stratum
- **WHEN** a stratum's clipped area is smaller than 1 m²
- **THEN** it receives no points and is listed as skipped

### Requirement: Transects and points along lines
The transects tool SHALL lay parallel lines at a spacing and bearing across a polygon, clipped to it, with a random start offset from the seed, and SHALL offer a zigzag option; it SHALL return each transect's length (m) and the total. The points-along-line tool SHALL place points every d meters of planar length from a start offset, keeping the last point only if it falls within the line, and SHALL report both planar and geodesic line lengths.

#### Scenario: Transect total length
- **WHEN** transects at 100 m spacing and bearing 90° cross a 1 km square
- **THEN** 10 transects of 1,000 m each (±1 mm) are returned with a 10,000 m total

#### Scenario: Last point rule
- **WHEN** points every 30 m are requested along a 100 m line
- **THEN** points fall at 0, 30, 60, and 90 m, and none at 120 m

### Requirement: Limits and export
Tools SHALL refuse requests above 100,000 points, 50,000 cells, or 10,000 transects with `LIMIT_EXCEEDED` naming the count the inputs imply, before allocation. Every result SHALL be exportable as GeoJSON, CSV, and KML with the seed, design, CRS, and citation carried as properties.

#### Scenario: Too many points
- **WHEN** a 5 m systematic grid is requested over 100 km²
- **THEN** the result is `LIMIT_EXCEEDED` stating 4,000,000 points against the 100,000 limit
