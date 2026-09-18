## Purpose

Computes Earth's magnetic field from official models so users can convert between true and magnetic bearings, with the model version, date validity, uncertainty, and unreliable-compass zones always visible.

## ADDED Requirements

### Requirement: Magnetic field elements
The domain SHALL compute, for a point (latitude, longitude, height above the WGS 84 ellipsoid or MSL with conversion) and a date, all seven field elements (declination D, inclination I, horizontal intensity H, north X, east Y, down Z, total intensity F) and their annual secular variation, using WMM2025 by default, with WMMHR2025 and IGRF-14 selectable. Heights SHALL be accepted from -1 km to 850 km per WMM's stated domain.

#### Scenario: Official test values
- **WHEN** the NCEI-published WMM2025 test points are evaluated
- **THEN** every element matches within the tolerances listed in the WMM2025 test-value file

#### Scenario: Model selection
- **WHEN** a user selects WMMHR2025 for a point
- **THEN** `meta.model` is `WMMHR2025` and the result includes WMMHR's stated uncertainty

### Requirement: Validity windows enforced
WMM2025 and WMMHR2025 SHALL accept dates from 2025.0 to 2030.0. IGRF-14 SHALL accept 1900.0 to 2030.0, with its definitive (DGRF) and provisional/predictive spans labeled. Dates outside a model's window SHALL return `OUT_OF_DOMAIN` (per data-assets), except that historical dates SHALL suggest IGRF-14.

#### Scenario: Historical declination
- **WHEN** a user requests WMM2025 declination for 1985-06-01
- **THEN** the result is `OUT_OF_DOMAIN` with a hint to use IGRF-14, which supports that date

### Requirement: Uncertainty and unreliable zones
Every declination result SHALL include the model uncertainty. For WMM2025, the tool SHALL compute the location-dependent declination uncertainty √(0.26² + (5417/H)²) degrees with H in nT. The result SHALL flag the **blackout zone** (H < 2,000 nT: compass unreliable) and **caution zone** (2,000 ≤ H < 6,000 nT). It SHALL also state that local crustal anomalies of several degrees are not modeled by WMM.

#### Scenario: Blackout zone
- **WHEN** declination is requested near the north magnetic pole where H < 2,000 nT
- **THEN** the result includes warning `COMPASS_BLACKOUT_ZONE` and the computed uncertainty

#### Scenario: Caution zone
- **WHEN** H is 4,500 nT at the point
- **THEN** the result includes warning `COMPASS_CAUTION_ZONE`

### Requirement: True ↔ magnetic bearing conversion
Tools SHALL convert true bearings to magnetic and back (magnetic = true − declination with east declination positive), and SHALL accept either a computed declination (model and date) or a user-entered variation (e.g. the chart's published epoch variation, "12°W"). Variation strings with E/W SHALL be parsed and normalized.

#### Scenario: Chart variation
- **WHEN** true course 090° is converted with variation `12°W`
- **THEN** magnetic course is 102°

#### Scenario: Chart vs model difference
- **WHEN** a user enters both a chart variation and requests the model value for today
- **THEN** the result shows both and their difference, noting that charts, runways, and navaids use an assigned epoch variation

### Requirement: Grid variation (grivation)
A tool SHALL compute grid variation (the angle between grid north of a chosen projection, e.g. UPS or UTM, and magnetic north) for polar and grid navigation.

#### Scenario: Grivation in UPS
- **WHEN** grivation is requested at 86° N in UPS north
- **THEN** the result combines grid convergence and declination and states the sign convention

### Requirement: Declination maps on the canvas
Magnetic tools SHALL be able to draw isogonic lines (lines of equal declination), the agonic line, and blackout/caution zones for the chosen model and date, computed on the device.

#### Scenario: Isogonic overlay
- **WHEN** a user enables the isogonic overlay for 2026-09-18
- **THEN** the canvas draws declination contours at a user-selected interval (default 2°) and shades the blackout and caution zones
