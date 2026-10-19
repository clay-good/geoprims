## Purpose

Tells a practitioner where water goes on their DEM: which cells are pits, which way each cell drains, how much area drains through it, where the channels are, what drains to a point, and how long the longest path is. Everything runs on the device from the same DEM sources as the terrain tools.

## ADDED Requirements

### Requirement: Depression removal
The fill tool SHALL remove depressions by Priority-Flood with an ε slope (default ε = 1e-5 vertical units) so every cell has a non-ascending path to a raster edge or a NoData boundary, or, when "breach" is chosen, by least-cost breaching with a maximum depth and length (defaults 10 m and 100 cells). It SHALL return the conditioned DEM, a count and map of changed cells, the total fill volume (m³, exact sum of raised cells × cell area), and the method name. NoData cells SHALL never be filled or crossed.

#### Scenario: Single bowl
- **WHEN** a 7 × 7 DEM has one interior pit 3 m below its spill elevation
- **THEN** every interior cell is raised to the spill elevation plus ε steps that drain to the edge, and the changed-cell count equals the bowl's cell count

#### Scenario: Pit at the edge stays
- **WHEN** the lowest cell of a depression lies on the raster edge
- **THEN** no cell is raised, because the edge is an outlet

### Requirement: Flow direction
The flow-direction tool SHALL compute D8 directions (steepest descent over eight neighbors, diagonal distance √2 × cell size, ties resolved to the lowest-numbered neighbor in ESRI order) and, as an option, D-infinity directions (Tarboton 1997, angle in radians counterclockwise from east). D8 results SHALL use the ESRI encoding (E 1, SE 2, S 4, SW 8, W 16, NW 32, N 64, NE 128, 0 for no outflow) by default, with WhiteboxTools and TauDEM encodings as named output options. A cell on the edge or beside NoData with no lower neighbor SHALL drain off the grid.

#### Scenario: Ramp
- **WHEN** a 10 × 10 DEM descends uniformly to the east
- **THEN** every cell's D8 direction is 1 (east) and the D∞ angle is 0 rad

#### Scenario: Encoding option
- **WHEN** WhiteboxTools encoding is requested for the same ramp
- **THEN** every cell's direction is 2 (east in that table), and the result names the encoding

### Requirement: Flow accumulation
The accumulation tool SHALL compute, for every cell, the number of upslope cells (u32) and the upslope area (count × cell area, m², with cell area from the DEM's CRS, or geodesic cell area for geographic DEMs). It SHALL use D8 by default or D∞ with proportional splitting. The result SHALL state that near the window edge the value is a lower bound.

#### Scenario: East ramp accumulation
- **WHEN** the 10 × 10 east ramp is accumulated
- **THEN** each row reads 1, 2, 3, …, 10 from west to east (counting the cell itself), as in the richdem reference behavior

#### Scenario: Conservation
- **WHEN** any D8 accumulation is computed
- **THEN** for every cell the count equals 1 plus the sum of the counts of the cells that point to it

### Requirement: Streams and order
The streams tool SHALL extract channel cells where accumulation (cells or area) meets a user threshold, assign Strahler order (headwaters 1; two equal orders meeting give order + 1; unequal give the larger), and return the network as GeoJSON lines with order, length (m), and the drainage area at each link's outlet. It SHALL report the number of links per order.

#### Scenario: Order rule
- **WHEN** two first-order links join and the result joins a second-order link
- **THEN** the first junction yields order 2 and the second remains order 2

#### Scenario: Threshold too high
- **WHEN** no cell meets the threshold
- **THEN** the result is an empty network with a note giving the window's maximum accumulation

### Requirement: Watershed from a pour point
The watershed tool SHALL snap the pour point by a named rule (nearest stream cell at the given threshold, the default; or highest accumulation within a distance), delineate every cell whose D8 path reaches the snapped cell, and return the basin polygon, area (geodesic, m² and the profile's area unit), the snapped outlet, relief (m), mean slope (%), the outlet's stream order, and the longest flow path. When the basin reaches the window edge it SHALL warn `WATERSHED_TOUCHES_EDGE`.

#### Scenario: Nested basins
- **WHEN** basins are delineated for a cell and for a cell upstream of it
- **THEN** the upstream basin lies entirely inside the downstream one

#### Scenario: Basin reaches the edge
- **WHEN** a pour point's basin includes window-edge cells
- **THEN** the result carries `WATERSHED_TOUCHES_EDGE` and the sentence says the true basin may continue beyond the data

#### Scenario: Pour point off the data
- **WHEN** the pour point lies outside the DEM window or on NoData
- **THEN** the result is `OUT_OF_DOMAIN` naming the window bounds

### Requirement: Longest flow path
The tool SHALL return the longest D8 flow path (m, summing cell-center distances with √2 diagonals) ending at a given outlet cell, as a GeoJSON line with its length and elevation drop.

#### Scenario: Ramp path
- **WHEN** the longest path is requested at the east edge of the 10-cell east ramp with 30 m cells
- **THEN** the length is 270 m (±0.001 m) and the path is the row

### Requirement: DEM provenance and surface-model caveat
Every hydrology result SHALL state the DEM source, resolution, vertical datum, the conditioning method and ε or breach limits, the direction encoding, and, for GLO-30, that it is a surface model so channels under canopy and culverts under roads are not represented. GLO-30 results SHALL carry the Copernicus attribution text.

#### Scenario: GLO-30 caveat
- **WHEN** a watershed is delineated on GLO-30
- **THEN** the result includes the attribution and the surface-model statement

### Requirement: Window limits
Hydrology tools SHALL refuse, with `LIMIT_EXCEEDED` before allocating, any window above 25,000,000 cells (10,000,000 under the mobile gate) and SHALL name the largest square window that fits. They SHALL report progress at least every 250 ms and cancel within 100 ms.

#### Scenario: Four GLO-30 tiles on a phone
- **WHEN** a 2° × 2° window (51.8 M cells) is requested
- **THEN** the result is `LIMIT_EXCEEDED` stating the cap and that one 1° tile (12.96 M cells) fits on desktop

### Requirement: Verification against reference implementations
Every hydrology tool SHALL be differentially tested against WhiteboxTools, and against richdem run as an external process, on at least 1,000 synthetic and 20 real DEM windows, matching fill elevations within 1e-6 m, directions exactly, accumulation counts exactly, and watershed cell sets exactly, except on documented tie cells.

#### Scenario: Oracle disagreement
- **WHEN** WhiteboxTools and richdem disagree on a cell
- **THEN** the cell is excluded from the vector with the reason recorded, and the derivation note lists it
