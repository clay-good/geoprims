## Purpose

Answers terrain questions on the device: elevation at a point, elevation along a path, slope and aspect, contours, whether two points can see each other over terrain, and what is visible from a point. It uses self-hosted open elevation data or the user's own DEM, with its accuracy stated.

## ADDED Requirements

### Requirement: Elevation data sources
Terrain tools SHALL use either the self-hosted Copernicus DEM GLO-30 tiles (`dem-glo30` asset) or a user-supplied DEM (GeoTIFF/COG, read locally). Every result SHALL state:
- the source
- the vertical datum (EGM2008 for GLO-30)
- the horizontal resolution
- the published vertical accuracy
- that GLO-30 is a surface model (it includes buildings and canopy)

Results using GLO-30 SHALL include the Copernicus attribution text and liability disclaimer required by its license.

#### Scenario: Attribution
- **WHEN** an elevation lookup uses GLO-30
- **THEN** the result includes the required Copernicus attribution text and states that the model is a DSM referenced to EGM2008

#### Scenario: Missing coverage
- **WHEN** a point lies in the ocean or in a region excluded from the public GLO-30 release
- **THEN** the result is `ASSET_UNAVAILABLE` naming the coverage gap, and suggests a user-supplied DEM

### Requirement: Elevation lookup and height conversion
The elevation tool SHALL return elevation at a point by bilinear (default) or nearest interpolation, and SHALL convert it to ellipsoidal height or another geoid via geodesy heights on request.

#### Scenario: Ellipsoidal conversion
- **WHEN** a user requests the terrain height as HAE
- **THEN** the tool adds the EGM2008 undulation and reports both values

### Requirement: Elevation profile
Given a path (two points or a polyline, geodesic edges), the profile tool SHALL sample elevation at a spacing no coarser than the DEM resolution. It SHALL return distance-elevation pairs, minimum, maximum, total ascent and descent, and maximum grade, and SHALL render a profile chart. An optional Earth-curvature-and-refraction adjustment SHALL be available for long profiles.

#### Scenario: Sampling density
- **WHEN** a 10 km profile is computed on GLO-30
- **THEN** at least 335 samples are returned (334 intervals of ≤ 30 m)

### Requirement: Slope, aspect, hillshade, and contours
Tools SHALL compute slope (degrees and percent), aspect (degrees from north, with flat areas flagged), and hillshade (sun azimuth and altitude inputs) using Horn's method. They SHALL account for geographic DEM cell sizes varying with latitude, and SHALL generate contours at an interval as vector lines. Derivatives SHALL match GDAL `gdaldem` within 0.01° for slope and aspect on projected test DEMs.

#### Scenario: Geographic cell size
- **WHEN** slope is computed on a geographic (degree-based) DEM at 60° N
- **THEN** east-west cell size is scaled by cos(latitude) (or the ellipsoidal equivalent) before computing gradients

#### Scenario: GDAL agreement
- **WHEN** slope and aspect are computed on the reference test DEM
- **THEN** every cell agrees with `gdaldem` within 0.01°

### Requirement: Terrain line of sight
Given an observer (position, height above ground) and a target (position, height above ground), the tool SHALL determine visibility along the geodesic profile, accounting for Earth curvature and refraction (k editable, default 0.13; radio k = 0.25 selectable). It SHALL report the obstruction point, the clearance margin (the minimum clearance of the sight line), and optionally the first Fresnel zone clearance for a frequency.

#### Scenario: Blocked by a ridge
- **WHEN** a ridge rises above the sight line between observer and target
- **THEN** the result is "not visible", with the obstruction's location, elevation, and the height the observer would need to see the target

#### Scenario: Fresnel check
- **WHEN** a 5.8 GHz link is checked
- **THEN** the result reports whether 60% of the first Fresnel zone is clear, and where it is not

### Requirement: Viewshed
Given an observer position and height, a maximum radius (up to 50 km in the web app), an optional target height, and a refraction coefficient, the viewshed tool SHALL compute visible and not-visible cells over the DEM using a documented algorithm (e.g. R2 or an XDraw variant, stated), in a worker with progress and cancel. It SHALL render the result over the map and export it as GeoTIFF and as GeoJSON polygons.

#### Scenario: Progress and cancel
- **WHEN** a 30 km viewshed is running
- **THEN** progress is reported at least every 250 ms, and cancel stops it within 100 ms

#### Scenario: Algorithm accuracy
- **WHEN** the viewshed of the reference DEM is compared with a brute-force per-cell line-of-sight computation
- **THEN** at least 99% of cells agree, and disagreements lie on visibility boundaries

### Requirement: Data volume and privacy limits
Terrain tools SHALL fetch only the 1° × 1° (or coarser) tiles needed, SHALL show the download size before fetching more than 50 MB, and SHALL work offline with terrain offline packs.

#### Scenario: Large-area confirmation
- **WHEN** a viewshed requires 80 MB of tiles not in cache
- **THEN** the user is asked to confirm the download size before fetching
