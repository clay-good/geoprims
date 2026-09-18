## Purpose

Organizes the full geoprims inventory into a navigable taxonomy with stable identifiers, an honest counting rule, and a lifecycle, so the catalog can grow to roughly 800 endpoints without padding or duplication.

## ADDED Requirements

### Requirement: Fixed top-level taxonomy
The catalog SHALL organize tools into these top-level domains: `geodesy` (coordinates, datums, projections, grid references, heights, geomagnetism), `navigation` (geodesics, rhumb lines, route geometry, 3D vectors, horizon and line of sight), `geometry` (computational geometry on the plane and the ellipsoid), `aviation` (atmosphere, airspeed, altimetry, wind, performance, flight planning), `drone` (photogrammetry, endurance and power, mission geometry, operations references), `survey` (traverse and COGO, instrument reductions, earthwork and grade, curves), `indexing` (discrete global grids, hierarchical cells, tiles), `raster` (imagery indices, terrain analysis), `time` (sun position, twilight and legal night definitions, time scales), and `units` (standalone unit and format conversions). Each domain SHALL be subdivided into groups; each tool belongs to exactly one group.

#### Scenario: Tool belongs to one group
- **WHEN** the catalog is built
- **THEN** every tool id's second segment names an existing group in its domain, and no id appears in two groups

### Requirement: Operations versus endpoints counting rule
The catalog SHALL distinguish **operations** (distinct mathematical functions with their own contract and vectors) from **endpoints** (addressable tool ids, including generated conversion pairs such as `geodesy.convert.dms-to-mgrs`). Generated endpoints SHALL be produced from a conversion graph over operations and SHALL NOT carry separate mathematics. Operations and generated endpoints share one naming scheme (conversions are named `a-to-b` either way); they are distinguished by the manifest's `composedOf` field, never by name. Public tool counts SHALL report both numbers.

#### Scenario: Generated pair shares math
- **WHEN** `geodesy.convert.dms-to-mgrs` is invoked
- **THEN** it executes the composition of the DMS parser and the MGRS forward operation, and its manifest lists both as `composedOf`

#### Scenario: Honest public count
- **WHEN** the home page shows the tool count
- **THEN** it displays both the operation count and the endpoint count of stable tools, computed from the build, with experimental tools counted separately

### Requirement: Generated endpoints must be meaningful
A generated conversion-pair endpoint SHALL be published only if the pair is a real user task (listed in the catalog's pair allow-list with a justification) and SHALL NOT be generated for every permutation automatically.

#### Scenario: Pair not allow-listed
- **WHEN** the conversion graph could produce `geodesy.convert.maidenhead-to-state-plane` but it is not in the allow-list
- **THEN** no endpoint, route, or search entry is generated for it; users can still chain the two tools manually

### Requirement: Aliases and search metadata
Every tool SHALL declare aliases and keywords that practitioners use (for example "E6B", "wind triangle", "WCA" for the wind-correction tool; "inverse", "distance between two points" for the geodesic inverse), and the catalog SHALL fail the build if two tools declare the same alias without a disambiguation note.

#### Scenario: Practitioner alias found
- **WHEN** a user types `wca` in the command palette
- **THEN** the wind-correction-angle tool is in the top 3 results

### Requirement: Lifecycle states
Every tool SHALL have one of the states `experimental`, `stable`, or `deprecated`. Experimental tools SHALL be visibly labeled in the web UI, SHALL be excluded from MCP search results and toolsets unless explicitly requested, and SHALL NOT be promoted to stable until they meet the stable-vector count and differential-test requirements.

#### Scenario: Experimental label
- **WHEN** a user opens an experimental tool
- **THEN** the page shows an "EXPERIMENTAL" badge and explains what that means

### Requirement: Related tools graph
Every tool SHALL list related tools (inverse operation, next logical step, alternative method), and the catalog SHALL verify that every inverse relation is declared symmetrically.

#### Scenario: Inverse symmetry
- **WHEN** `geodesy.utm.forward` declares `geodesy.utm.inverse` as its inverse
- **THEN** the build fails unless `geodesy.utm.inverse` declares `geodesy.utm.forward` as its inverse

### Requirement: Catalog export
The build SHALL publish the full catalog as a versioned JSON file at a stable URL (`/catalog/v1.json`) containing every manifest, for use by the MCP server and search engines. Within `v1`, fields MAY be added but SHALL NOT be removed or renamed; breaking changes publish `/catalog/v2.json` alongside `v1` for at least 12 months.

#### Scenario: Catalog available offline
- **WHEN** the PWA is installed and the device is offline
- **THEN** the catalog JSON is available from the cache and the command palette works
