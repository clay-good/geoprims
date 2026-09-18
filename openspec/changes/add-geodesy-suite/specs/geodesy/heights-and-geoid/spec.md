## Purpose

Makes heights unambiguous by converting between ellipsoidal heights (what GNSS reports), orthometric heights (what maps and charts call elevation or MSL), and heights above ground, using named geoid models with stated interpolation and model accuracy.

## ADDED Requirements

### Requirement: Geoid undulation lookup
The domain SHALL return the geoid undulation N at a point for each supported model: EGM96 (15′ grid), EGM2008 (5′, 2.5′, and 1′ grids), and GEOID18 (where covered). The user selects the model, or it defaults to the best available model for the point. Interpolation SHALL be bicubic by default (bilinear selectable). Each result SHALL state the model, grid, interpolation method, the interpolation error bound for that grid (e.g. EGM2008-2.5′ cubic ≤ 0.031 m max versus the full model), and the model's own accuracy statement.

#### Scenario: Model and error stated
- **WHEN** N is requested at a point with EGM2008 2.5′ cubic interpolation
- **THEN** the result includes N in meters, `meta.model` "EGM2008", grid "2.5′", method "cubic", and the interpolation bound

#### Scenario: GEOID18 coverage
- **WHEN** GEOID18 is requested for a point in France
- **THEN** the result is `OUT_OF_DOMAIN` naming GEOID18's coverage (CONUS, Puerto Rico, US Virgin Islands) and suggesting EGM2008

#### Scenario: Differential check
- **WHEN** 10,000 random points are evaluated with EGM2008 2.5′ cubic
- **THEN** results agree with GeographicLib `GeoidEval` using the same grid within 1 mm

### Requirement: Ellipsoidal ↔ orthometric heights
Tools SHALL convert ellipsoidal height h to orthometric height H (H = h − N) and back with the chosen geoid, and SHALL require the horizontal datum of h to be consistent with the geoid model (e.g. GEOID18 expects NAD83(2011) ellipsoid heights; EGM2008 expects WGS 84). When inconsistent, the tool SHALL either apply the horizontal/ellipsoidal frame transformation (reporting it) or warn `FRAME_MISMATCH`.

#### Scenario: NAVD 88 from GNSS
- **WHEN** a NAD83(2011) ellipsoid height is converted to orthometric with GEOID18
- **THEN** the result is labeled NAVD 88 orthometric height

#### Scenario: Frame mismatch
- **WHEN** a WGS 84 (G2296) ellipsoid height is converted with GEOID18 without allowing a frame transformation
- **THEN** the result includes `FRAME_MISMATCH` stating the expected frame and the approximate vertical impact

### Requirement: Height reference converter
A tool SHALL convert a height among HAE (height above ellipsoid), MSL/orthometric (per chosen geoid), AGL (given ground elevation, either user-supplied or from the terrain asset with its accuracy), and pressure/flight-level references by linking to the aviation altimetry tools. The result SHALL display a vertical diagram of all references at the point.

#### Scenario: Drone altitude references
- **WHEN** a drone reports 120 m HAE at a point where N = -33.2 m and terrain is 250 m MSL
- **THEN** the tool reports 153.2 m MSL and -96.8 m AGL, flags the negative AGL as below ground (`BELOW_TERRAIN`), and draws the vertical diagram

### Requirement: Geoid model comparison
A tool SHALL compare two geoid models at a point or along a profile (e.g. EGM96 vs EGM2008) and report the difference.

#### Scenario: EGM96 vs EGM2008
- **WHEN** the comparison is run at a point
- **THEN** the result reports both undulations and the difference with each model's accuracy

### Requirement: NSRS 2022 heights labeled beta
GEOID2022 and NAPGD2022 heights SHALL be offered only when their assets are present and SHALL carry `NON_OFFICIAL_DATUM` until adoption.

#### Scenario: GEOID2022 label
- **WHEN** a GEOID2022 height is computed before official adoption
- **THEN** the result carries `NON_OFFICIAL_DATUM`
