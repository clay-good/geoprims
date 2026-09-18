## Purpose

Computes vegetation, water, burn, and built-up spectral indices, and user-defined band math, on single reflectance values or on the user's own imagery files, entirely on the device with correct sensor scaling.

## ADDED Requirements

### Requirement: Spectral index catalog
The domain SHALL compute at least the following, with formulas and citations:

| Index | Formula |
|---|---|
| NDVI | (NIR − Red) / (NIR + Red) |
| NDWI (McFeeters 1996, open water) | (Green − NIR) / (Green + NIR) |
| NDWI (Gao 1996, vegetation water) | (NIR − SWIR1) / (NIR + SWIR1) |
| MNDWI | (Green − SWIR1) / (Green + SWIR1) |
| EVI | 2.5 · (NIR − Red) / (NIR + 6·Red − 7.5·Blue + 1) |
| EVI2 | 2.5 · (NIR − Red) / (NIR + 2.4·Red + 1) |
| SAVI | (1 + L) · (NIR − Red) / (NIR + Red + L), default L = 0.5 |
| NDBI | (SWIR1 − NIR) / (SWIR1 + NIR) |
| NBR | (NIR − SWIR2) / (NIR + SWIR2) |
| dNBR | NBR_pre − NBR_post, with burn-severity class ranges labeled as reference |

The two NDWI definitions SHALL be separate tools with distinct names.

#### Scenario: NDVI value
- **WHEN** NIR = 0.45 and Red = 0.08 (surface reflectance)
- **THEN** NDVI ≈ 0.6981

#### Scenario: NDWI disambiguation
- **WHEN** a user searches "NDWI"
- **THEN** both the McFeeters and the Gao tools are shown, each describing what it measures

#### Scenario: Zero denominator
- **WHEN** NIR + Red = 0
- **THEN** the result for that pixel is no-data, and single-value mode returns `INVALID_INPUT` explaining the zero denominator

### Requirement: Reflectance scaling and sensor presets
Index tools SHALL require reflectance on a 0–1 scale. They SHALL provide sensor presets that map band names and apply scaling:
- **Sentinel-2 L2A:** B2 blue, B3 green, B4 red, B8 NIR, B8A narrow NIR, B11 SWIR1, B12 SWIR2. Reflectance = (DN − 1000) / 10000 for processing baseline 04.00 and later; DN / 10000 before it, with the baseline selectable.
- **Landsat 8/9 Collection 2 Level-2:** B2–B7. Reflectance = DN · 0.0000275 − 0.2.

Values outside a plausible range (e.g. reflectance > 1.5 or < -0.2 after scaling) SHALL trigger `SUSPECT_SCALING`.

#### Scenario: Raw DN entered
- **WHEN** NIR = 3,500 and Red = 800 are entered as reflectance
- **THEN** the result includes `SUSPECT_SCALING` and suggests a sensor preset

#### Scenario: Sentinel-2 offset
- **WHEN** a Sentinel-2 L2A preset with baseline 04.00 is applied to DN 1,450
- **THEN** reflectance = 0.045

### Requirement: Local raster processing
Tools SHALL accept local GeoTIFF/COG files (single multiband or separate band files) up to 500 megapixels per band in the web app. They SHALL read them in a worker without uploading, compute the index per pixel with no-data handling, and output a preview with a legend, a histogram, and summary statistics. The result SHALL be downloadable as a GeoTIFF preserving georeferencing.

#### Scenario: Mismatched band grids
- **WHEN** two band files have different extents or resolutions
- **THEN** the tool reports `GRID_MISMATCH` with both grids and offers nearest or bilinear resampling to one of them

#### Scenario: Output georeferencing
- **WHEN** an NDVI GeoTIFF is exported
- **THEN** it has the same CRS, geotransform, and no-data value as the input

### Requirement: Safe band-math expressions
A band-math tool SHALL evaluate user expressions over named bands with arithmetic, comparison, conditional, and a whitelist of functions (abs, sqrt, log, exp, min, max, clamp). It SHALL parse expressions into an AST and evaluate them in the core, never with JavaScript `eval`. It SHALL reject unknown identifiers and cap expression size at 1,000 nodes.

#### Scenario: Unknown identifier
- **WHEN** the expression references `window.location`
- **THEN** it is rejected with `INVALID_INPUT` naming the unknown identifier

### Requirement: Index legends are accessible
Index previews SHALL use perceptually uniform, color-vision-deficiency-safe palettes (diverging for NDVI-type indices), with legend ticks and a numeric readout under the cursor.

#### Scenario: Cursor readout
- **WHEN** the cursor hovers over the NDVI preview
- **THEN** the pixel's index value and coordinates are shown
