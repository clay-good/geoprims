## Purpose

Gives every geoprims tool one unit system with exact, cited conversion constants, unambiguous parsing of unit-tagged inputs, and consistent output formatting, eliminating the unit errors that dominate aviation and survey calculation mistakes.

## ADDED Requirements

### Requirement: Canonical SI internals
All tool computations SHALL operate on canonical units: meters, radians internally (degrees at the boundary), seconds, kilograms, kelvin, pascals, meters per second, joules, watts. Unit conversion SHALL occur only at the tool boundary.

#### Scenario: Mixed-unit inputs
- **WHEN** a density altitude tool receives field elevation `5,280 ft`, temperature `30 °C`, and altimeter `29.92 inHg`
- **THEN** the tool converts each to canonical units before computation and the result equals the result for `1609.344 m`, `303.15 K`, `101,320.76 Pa` (29.92 × 3386.389 Pa, ±0.01 Pa)

### Requirement: Exact defined constants
The unit registry SHALL use these exact definitions and SHALL cite their source (NIST SP 811 / NIST Handbook 44 / BIPM SI Brochure / ICAO Annex 5):

| Unit | Definition |
|---|---|
| international foot `ft` | 0.3048 m exactly |
| US survey foot `ftUS` | 1200/3937 m exactly |
| international nautical mile `NM` | 1852 m exactly |
| statute mile `mi` | 1609.344 m exactly |
| knot `kt` | 1852/3600 m/s exactly |
| pound-mass `lb` | 0.45359237 kg exactly |
| US gallon `galUS` | 3.785411784 L exactly |
| inch of mercury `inHg` | 3386.389 Pa (conventional, 0 °C, standard gravity; cited) |
| hectopascal `hPa` / millibar `mbar` | 100 Pa exactly |
| pound per square inch `psi` | 6894.757293168361 Pa (derived exactly from lb, ft, g0) |
| standard gravity `g0` | 9.80665 m/s² exactly |
| degree Fahrenheit | T[K] = (T[°F] + 459.67) × 5/9 exactly |
| degree Celsius | T[K] = T[°C] + 273.15 exactly |

#### Scenario: Nautical mile exact
- **WHEN** 1 NM is converted to meters
- **THEN** the result is exactly 1852

#### Scenario: Survey foot distinct
- **WHEN** 1,000,000 ftUS and 1,000,000 ft are converted to meters
- **THEN** the results differ by 0.6096012 m (±1e-9 m), the 2 ppm difference

### Requirement: US survey foot handled as legacy
The US survey foot SHALL be available for reading and writing legacy data but SHALL NOT be the default for any tool. Any tool whose output is in `ftUS` SHALL attach warning `LEGACY_UNIT` noting its deprecation by NIST/NOAA effective January 1, 2023. State Plane tools SHALL expose the unit per zone definition explicitly.

#### Scenario: Legacy unit warning
- **WHEN** a user selects `ftUS` for a State Plane output
- **THEN** the result includes warning `LEGACY_UNIT`

### Requirement: Angular units are explicit and mils are disambiguated
Supported angle units SHALL include degrees, radians, gradians (gon), arcminutes, arcseconds, and mils. Mils SHALL require an explicit variant: `mil-nato` (6400 per turn), `mil-warsaw` (6000 per turn), `mil-sweden` (6300 per turn), or `mrad` (true milliradian, 2000π per turn). A bare `mil` SHALL be rejected with `UNIT_MISMATCH` and a hint listing the variants.

#### Scenario: Bare mil rejected
- **WHEN** an input is `1600 mil`
- **THEN** the tool returns `UNIT_MISMATCH` listing `mil-nato`, `mil-warsaw`, `mil-sweden`, `mrad`

### Requirement: Unit-tagged value parsing
Any numeric input SHALL accept either a bare number (interpreted in the field's declared default unit, which the UI always displays) or a string with a unit suffix from the registry, including common aliases (`kts`, `knots`, `nm`, `NM`, `nmi`, `'` for feet only where unambiguous, `°`, `deg`, `°C`, `degC`, `C`, `"Hg`, `inhg`, `mb`, `hPa`). Parsing SHALL be case-sensitive where case distinguishes units (`mm` vs `Mm`, `mbar` vs `Mbar` rejected). Thousands separators `,` and `_` SHALL be accepted; a decimal comma SHALL be accepted only when the UI's number-format setting is set to decimal comma.

#### Scenario: Aliased unit
- **WHEN** a speed input is `145 kts`
- **THEN** it parses as 145 knots

#### Scenario: Quantity mismatch
- **WHEN** a speed field receives `145 ft`
- **THEN** the tool returns `UNIT_MISMATCH` stating that the field expects a speed

#### Scenario: Ambiguous nm
- **WHEN** a distance field receives `12 nm`
- **THEN** it parses as 12 nautical miles and the result includes warning `UNIT_ASSUMED` stating that lowercase `nm` was read as nautical miles, not nanometers

### Requirement: Temperature differences are distinct from temperatures
The unit system SHALL distinguish absolute temperatures from temperature differences. A temperature difference of 1 °C SHALL convert to 1 K and 1.8 °F-difference; ISA deviation fields SHALL be typed as temperature differences.

#### Scenario: ISA deviation in Fahrenheit
- **WHEN** an ISA deviation of +18 °F (difference) is entered
- **THEN** it converts to +10 K

### Requirement: Output unit selection
Every tool with quantity outputs SHALL allow the caller to request output units per field or via a unit profile (`si`, `aviation` [ft, kt, NM, inHg or hPa selectable, °C], `us-customary`, `survey-metric`, `survey-us`). The machine-readable result SHALL always state the unit alongside each numeric value.

#### Scenario: Aviation profile
- **WHEN** a geodesic inverse is requested with profile `aviation`
- **THEN** distance is returned in NM and the result states `"unit": "NM"`

### Requirement: Standalone unit conversion tools
The `units` domain SHALL expose the unit registry as tools: one operation per quantity family (length, area, volume, mass, speed, vertical speed, acceleration, pressure, temperature, temperature difference, angle, angular rate, time, energy, power, electric charge (mAh↔Ah↔C, and to Wh with a stated voltage), fuel volume↔mass (with a stated density), density, frequency, data rate, and slope/grade (percent, ratio, degrees, per mille)). Allow-listed pair endpoints (for example `units.speed.kt-to-mph`, `units.pressure.inhg-to-hpa`, `units.length.ftus-to-m`) SHALL be generated from these operations per the tool-catalog counting rule.

#### Scenario: Pair endpoint uses exact constant
- **WHEN** `units.speed.kt-to-mph` converts 100 kt
- **THEN** the result is 115.07794480235425 mph (1852/1609.344 × 100, shortest round-trip)

#### Scenario: Fuel mass needs density
- **WHEN** a user converts 50 galUS of fuel to lb without choosing a fuel type or density
- **THEN** the tool returns `INVALID_INPUT` for `/density`, offering presets (avgas 100LL 6.0 lb/gal, Jet-A 6.7 lb/gal nominal at 15 °C) labeled as nominal planning values
