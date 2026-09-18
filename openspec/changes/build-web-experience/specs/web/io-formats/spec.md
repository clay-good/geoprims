## Purpose

Lets users bring data into geoprims tools and take results out in standard formats, entirely on-device, including batch processing of spreadsheets of inputs.

## ADDED Requirements

### Requirement: Supported import formats
The app SHALL import, by file picker, drag-and-drop, or paste: GeoJSON (RFC 7946), KML 2.2 / KMZ, GPX 1.1, CSV/TSV (with column mapping), WKT and hex-encoded WKB/EWKB, and plain coordinate lists in any supported coordinate notation. Declared maximum sizes SHALL be 50 MB per file and 1,000,000 vertices per import; larger files SHALL be rejected with the limit stated.

#### Scenario: Drop a GPX track
- **WHEN** a user drops a GPX file with one track onto a polyline-input tool
- **THEN** the track's points populate the input and the canvas shows the track

#### Scenario: Oversized file
- **WHEN** a user drops a 120 MB GeoJSON file
- **THEN** the app refuses it before parsing and states the 50 MB limit

### Requirement: Coordinate order and CRS safety
GeoJSON SHALL be read as `[lon, lat]` per RFC 7946. CSV imports SHALL require the user to confirm which columns are latitude and longitude (auto-suggested from headers) and SHALL warn when values look swapped (for example, "latitude" column values outside ±90). Files declaring a non-WGS84 CRS (legacy GeoJSON `crs` member, KML is always WGS84, CSV with a user-chosen CRS) SHALL be transformed with the declared CRS or rejected if the CRS is unsupported.

#### Scenario: Swapped CSV columns
- **WHEN** a CSV's column labeled `lat` contains values like -105.2
- **THEN** the import dialog warns that the latitude column contains values outside ±90 and offers to swap

### Requirement: Supported export formats
Results SHALL be exportable as JSON (full result with `meta`), GeoJSON (geographic outputs with result properties), KML, GPX (routes and waypoints), CSV (tabular outputs and batch results), WKT, plain text, and a Markdown or HTML "calculation sheet" (inputs, outputs, formula, references, provenance, timestamp in ISO 8601 UTC, and disclaimer) suitable for field notes or audit files.

#### Scenario: Calculation sheet
- **WHEN** a surveyor exports a traverse closure as a calculation sheet
- **THEN** the file includes every input, adjusted coordinates, misclosure, precision ratio, method (e.g. Bowditch), tool and core versions, and references

### Requirement: Batch mode over CSV
Every tool that supports batch invocation SHALL offer a batch mode: import a CSV, map columns to inputs (with per-column units), run up to 100,000 rows off the main thread with progress and cancel, preview errors per row, and export results joined to the original columns.

#### Scenario: Batch with errors
- **WHEN** a 10,000-row batch contains 12 invalid rows
- **THEN** 9,988 rows produce results, the 12 errors are listed with row numbers and messages, and the export includes an `error` column

### Requirement: Clipboard integration
Copying a result SHALL offer formats: value only, value with unit, JSON, and a CLI or MCP invocation. Pasting into a coordinate field SHALL detect notation (per command-palette paste-to-detect).

#### Scenario: Copy value with unit
- **WHEN** a user copies a distance result as "value with unit"
- **THEN** the clipboard contains the displayed value and unit, for example `3,012.4 NM`

### Requirement: Safe parsing
Parsers SHALL run in a worker, SHALL not fetch any resource referenced by a file (KML NetworkLinks, external icons, XML external entities), SHALL strip scripts and HTML from descriptions before display, and SHALL report any ignored elements.

#### Scenario: Malicious KML description
- **WHEN** a KML placemark description contains a `<script>` tag
- **THEN** the description is displayed as inert text and the import report notes the removal

### Requirement: Geometry validation on import
Imported polygons SHALL be checked for self-intersection, unclosed rings, duplicate consecutive vertices, and ring orientation; the user SHALL be offered automatic repair (close rings, remove duplicates, reorient per RFC 7946) with a report, and invalid geometry SHALL never be passed silently to a tool.

#### Scenario: Unclosed ring
- **WHEN** an imported WKT polygon's ring is not closed
- **THEN** the import offers to close it and reports the repair
