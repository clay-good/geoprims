## Purpose

Predicts tide heights and the times of high and low water from harmonic constituents the user pastes from NOAA, and converts heights between tidal datums from a pasted datum sheet. Nothing is fetched.

## ADDED Requirements

### Requirement: Constituent and datum paste parsing
The tide tool SHALL parse the NOAA station harmonic-constituent table (rows of number, name, amplitude, phase, speed, description) and the `harcon.json` array, in feet or meters, with phases in local standard time or GMT as the paste states, and SHALL parse the NOAA datum sheet table and `datums.json`. It SHALL list any of the 37 NOAA constituents that are missing and SHALL require an explicit "partial constituents" acknowledgment to run without them.

#### Scenario: JSON paste
- **WHEN** the `HarmonicConstituents` array for a station is pasted with `units=metric`
- **THEN** 37 constituents load with amplitudes in meters and the phase reference the paste declares, and the station's time reference is shown

#### Scenario: Missing constituents
- **WHEN** a paste holds 20 of the 37 rows
- **THEN** the tool lists the 17 missing by name and runs only after the acknowledgment, carrying `PARTIAL_CONSTITUENTS`

### Requirement: Harmonic prediction
The tool SHALL predict h(t) = H₀ + Σ f·H·cos(a·t + (V₀+u) − κ) using Schureman's node factors f and equilibrium arguments V₀+u for every constituent, evaluated at the middle of the prediction year (NOAA's practice) or at each instant (selectable, difference shown), with speeds from SP 98 Table 2. It SHALL return a height series at the chosen interval (default 6 minutes), the highs and lows with times (found by bracketing to 1 second), in the station's datum and local standard time (daylight time only when asked, by the tzdb zone), over a window of at most 31 days. Every result SHALL state the f and u year and that NOAA's published predictions are authoritative.

#### Scenario: NOAA parity
- **WHEN** a day is predicted for a NOAA station from its pasted constituents and datum sheet
- **THEN** hourly heights and the high and low times and heights match NOAA's predictions for that day within 0.02 ft and 2 minutes

#### Scenario: Node factor check
- **WHEN** f and V₀+u are computed for M2, S2, N2, K1, and O1 for any year 1990–2040
- **THEN** they match the reference implementations within 1e-6 and 0.001°

#### Scenario: Window too long
- **WHEN** a 90-day window is requested
- **THEN** the result is `LIMIT_EXCEEDED` naming the 31-day limit

### Requirement: Tidal datum conversion
The datums tool SHALL convert a height between any two datums on a pasted datum sheet (STND, MHHW, MHW, MTL, MSL, MLW, MLLW, NAVD88, LAT, HAT, and others the sheet lists) by subtraction, SHALL draw the datum stack, and SHALL refuse a datum the sheet lacks.

#### Scenario: MLLW to NAVD88
- **WHEN** a Boston sheet gives MLLW 1.074 m and NAVD88 2.752 m above STND and a height of 3.000 m MLLW is converted
- **THEN** the height is 1.322 m NAVD88 (±0.001 m)

#### Scenario: Datum not on sheet
- **WHEN** a conversion to LAT is requested and the sheet has no LAT row
- **THEN** the result is `INVALID_INPUT` for `/to` naming the datums the sheet has

### Requirement: Tide results feed other tools
A predicted height at a chosen time SHALL be sendable to the under-keel-clearance tool and to the datums tool, with the station's datum carried.

#### Scenario: Send to clearance
- **WHEN** the reader sends the 14:30 predicted height to under-keel clearance
- **THEN** the clearance tool opens with that tide height and datum filled
