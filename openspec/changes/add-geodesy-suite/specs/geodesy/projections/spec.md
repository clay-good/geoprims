## Purpose

Converts between geographic coordinates and projected grids (UTM, UPS, State Plane, and the common engineering and web projections), with grid convergence and scale factor, at accuracy matching the reference implementations.

## ADDED Requirements

### Requirement: UTM forward and inverse
The domain SHALL convert geographic coordinates to UTM (zone, hemisphere, easting, northing) and back on any supported ellipsoid, using the 6th-order Krüger series (Karney 2011). Accuracy SHALL be 5 nm (nanometers) or better within 3,900 km of the central meridian. The UTM domain is 80° S to 84° N. A tool SHALL also allow forcing a neighboring zone (up to ±3 zones, with the result's scale factor and a `NONSTANDARD_ZONE` warning).

#### Scenario: Pittsburgh in zone 17
- **WHEN** (40.446111°, -79.982222°) WGS 84 is converted to UTM
- **THEN** the result is zone 17N, easting ≈ 586,309.953 m, northing ≈ 4,477,770.428 m (±1 mm)

#### Scenario: Forced zone
- **WHEN** the same point is forced into zone 18
- **THEN** the result is valid with warning `NONSTANDARD_ZONE` and the larger point scale factor reported

#### Scenario: Beyond UTM domain
- **WHEN** latitude 84.5° N is converted to UTM
- **THEN** the result is `OUT_OF_DOMAIN` with a hint to `geodesy.ups.forward`

### Requirement: UTM zone rules including exceptions
Zone lookup SHALL apply the standard 6° zones plus the Norway exception (zone 32V widened to 3°–12° E; 31V narrowed to 0°–3° E) and the Svalbard exceptions (band X: zones 31X 0°–9° E, 33X 9°–21° E, 35X 21°–33° E, 37X 33°–42° E; zones 32X, 34X, 36X do not exist). Points exactly on a zone boundary SHALL be assigned to the eastern zone.

#### Scenario: Norway exception
- **WHEN** zone lookup runs for (60° N, 5° E)
- **THEN** the zone is 32 (not 31)

#### Scenario: Svalbard exception
- **WHEN** zone lookup runs for (78° N, 10° E)
- **THEN** the zone is 33

### Requirement: UPS for polar regions
The domain SHALL convert to and from Universal Polar Stereographic (north for latitude ≥ 84° N, south for latitude < 80° S, scale factor 0.994, false easting and northing 2,000,000 m).

#### Scenario: UPS north
- **WHEN** (85° N, 0° E) is converted to UPS
- **THEN** easting is 2,000,000 m and northing ≈ 1,444,542.609 m (±1 mm)

### Requirement: State Plane Coordinate Systems
The domain SHALL convert to and from every SPCS83 zone (124 zones) and every SPCS2022 zone published by NGS (beta until official adoption), including zone lookup by point and by state/county, and SHALL output in the zone's defined unit (meters, international feet, or US survey feet for SPCS83 zones where state law specifies it), with `LEGACY_UNIT` for US survey feet. Accuracy SHALL match NGS NCAT within 1 mm.

#### Scenario: Pennsylvania South in US survey feet
- **WHEN** (40.446111°, -79.982222°) NAD83 is converted to SPCS83 Pennsylvania South (US survey feet)
- **THEN** easting ≈ 1,347,294.025 ftUS, northing ≈ 413,222.374 ftUS (±0.003 ftUS), and the result includes `LEGACY_UNIT`

#### Scenario: Point outside zone
- **WHEN** a point in Ohio is converted to a Pennsylvania zone
- **THEN** the result is computed with warning `OUTSIDE_ZONE_EXTENT` and the correct zone is suggested

### Requirement: General projection methods with custom parameters
The domain SHALL provide forward and inverse tools with user-defined parameters for: Transverse Mercator (series and exact), Lambert Conformal Conic (1SP and 2SP), Albers Equal Area, Hotine Oblique Mercator (variants A and B), Polar Stereographic (variants A and B), Azimuthal Equidistant, Equidistant Cylindrical, Orthographic, Gnomonic, and Web Mercator (EPSG:3857). Parameter names SHALL follow EPSG Guidance Note 7-2.

#### Scenario: Web Mercator latitude limit
- **WHEN** latitude 86° is converted to Web Mercator
- **THEN** the result is `OUT_OF_DOMAIN` stating the limit ±85.05112878°

#### Scenario: Gnomonic horizon
- **WHEN** a point 90° or more from the gnomonic center is projected
- **THEN** the result is `OUT_OF_DOMAIN` stating that the gnomonic projection shows less than a hemisphere

### Requirement: CRS registry lookup and transform
A tool SHALL search the curated CRS registry by EPSG code, name, or location (returning CRSs whose area of use contains the point), and a transform tool SHALL convert coordinates between any two registry CRSs, composing projection and datum steps and reporting each step and the total accuracy.

#### Scenario: Location-based CRS suggestions
- **WHEN** a user asks for CRSs covering a point in Denver
- **THEN** the results include UTM 13N (NAD83 and WGS 84), SPCS83 Colorado Central, and SPCS2022 zones covering the point, each with its status

#### Scenario: Composite path reported
- **WHEN** SPCS83 Colorado Central (NAD83(2011)) is transformed to UTM 13N (WGS 84 (G2296))
- **THEN** the result lists steps inverse projection → frame transformation at the stated epoch → forward projection, with the accuracy of each

### Requirement: Grid convergence and point scale factor
Tools SHALL compute grid convergence (the angle from true north to grid north, with its sign convention stated) and point scale factor for any supported projection at a point, plus the arc-to-chord (t−T) correction for a line in TM and LCC.

#### Scenario: UTM convergence
- **WHEN** convergence and scale factor are computed at (40.446111°, -79.982222°) in UTM 17N
- **THEN** convergence ≈ 0.66031° (grid north east of true north, since the point is east of the central meridian) and scale factor ≈ 0.99969170 (±1e-8)

### Requirement: Round-trip and differential accuracy
Every forward/inverse projection pair SHALL round-trip within 1 nm (1e-9 m) inside its declared domain, and SHALL agree with PROJ 9.x (and GeographicLib for TM/UPS) within 1 mm on 10,000 random points per projection.

#### Scenario: Differential test
- **WHEN** the differential suite runs LCC 2SP against PROJ on 10,000 points
- **THEN** all points agree within 1 mm
