## Purpose

Encodes and decodes the alphanumeric grid references used in military, emergency-response, amateur-radio, and aviation work (MGRS, USNG, Maidenhead, GARS, and GEOREF), with correct precision semantics and polar handling.

## ADDED Requirements

### Requirement: MGRS encode and decode
The domain SHALL encode geographic coordinates to MGRS at precisions from grid zone only (e.g. `17T`) through 100 km, 10 km, 1 km, 100 m, 10 m, 1 m, 0.1 m, 0.01 m, and 0.001 m, and decode MGRS strings with or without spaces, per NGA.STND.0037 and NGA.SIG.0012. Encoding SHALL truncate (not round) to the requested precision, and decoding SHALL return both the south-west corner and the center of the referenced square, with the square size.

#### Scenario: Pittsburgh at 1 m
- **WHEN** (40.446111°, -79.982222°) is encoded to MGRS at 1 m precision
- **THEN** the result is `17TNE8630977770`

#### Scenario: Truncation not rounding
- **WHEN** the same point is encoded at 10 m precision
- **THEN** the result is `17TNE86307777`

#### Scenario: Decode returns square
- **WHEN** `17TNE8630977770` is decoded
- **THEN** the result gives the south-west corner, the center (offset 0.5 m in easting and northing), and the square size of 1 m

### Requirement: MGRS polar, exception, and lettering rules
MGRS SHALL use UPS with bands A, B (south) and Y, Z (north) beyond the UTM limits, SHALL honor the Norway and Svalbard zone exceptions, SHALL use the "AA" 100 km lettering scheme for WGS 84/GRS 80/NAD83 and the "AL" scheme for legacy ellipsoids (Clarke 1866, Bessel, and others as per NGA), and SHALL reject 100 km square identifiers that do not exist in the given zone and band.

#### Scenario: Svalbard grid zone
- **WHEN** (78° N, 10° E) is encoded at 100 km precision
- **THEN** the grid zone designator is `33X`

#### Scenario: South polar
- **WHEN** (-80.5°, 0°) is encoded
- **THEN** the grid zone designator is `B` (UPS south, eastern half)

#### Scenario: Invalid square
- **WHEN** an MGRS string names a 100 km square letter pair that does not occur in its zone and band
- **THEN** the decoder returns `INVALID_INPUT` naming the square

### Requirement: Band-boundary tolerance
The decoder SHALL accept an MGRS reference whose latitude band letter is off by one band when the point lies within 5 nm of the band boundary (matching GeographicLib behavior), returning warning `BAND_ADJUSTED`.

#### Scenario: Neighboring band accepted
- **WHEN** a reference computed exactly on a band boundary uses the southern band letter
- **THEN** it decodes with warning `BAND_ADJUSTED`

### Requirement: USNG
The domain SHALL encode and decode USNG (MGRS-equivalent on NAD83/WGS 84, written with spaces, e.g. `17T NE 86309 77770`), and SHALL accept truncated USNG references in a known local zone context (e.g. `NE 863 777` with a user-set grid zone).

#### Scenario: Space-delimited output
- **WHEN** the Pittsburgh point is encoded as USNG at 1 m
- **THEN** the result is `17T NE 86309 77770`

### Requirement: Maidenhead locator
The domain SHALL encode and decode Maidenhead locators at 2, 4, 6, 8, and 10 characters (field, square, subsquare, extended square, extended subsquare), case-insensitive on input, conventional case on output (upper fields, lower subsquares), and SHALL handle the +90° latitude and +180° longitude edges by clamping into the last cell.

#### Scenario: Six-character locator
- **WHEN** (40.446111°, -79.982222°) is encoded at 6 characters
- **THEN** the result is `FN00ak`

#### Scenario: Edge clamp
- **WHEN** latitude 90° is encoded
- **THEN** the locator is in field row `R` and decodes back to the northernmost cell

### Requirement: GARS and GEOREF
The domain SHALL encode and decode GARS (30′ cells, 15′ quadrants, and 5′ keypads) and World Geographic Reference System (GEOREF) at all standard precisions, returning cell bounds.

#### Scenario: GARS keypad
- **WHEN** a point is encoded as GARS at 5′ precision
- **THEN** the result is a 7-character code (3 digits, 2 letters, quadrant digit, keypad digit) and the cell bounds

### Requirement: Grid overlays on the canvas
Each grid reference tool SHALL be able to draw the grid (UTM zones and bands, MGRS 100 km squares and finer lines at zoom, Maidenhead fields and squares, GARS cells) around the result, and highlight the referenced square.

#### Scenario: MGRS overlay
- **WHEN** an MGRS result at 1 km precision is shown
- **THEN** the canvas highlights the 1 km square and draws the surrounding 1 km and 100 km grid lines
