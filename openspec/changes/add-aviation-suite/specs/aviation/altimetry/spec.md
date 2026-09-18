## Purpose

Computes pressure altitude, density altitude, flight levels, altimeter settings, and cold-temperature altitude corrections exactly. Each exact value is shown next to the rules of thumb pilots learn, so the difference is visible.

## ADDED Requirements

### Requirement: Pressure altitude from altimeter setting
Given field (or indicated) elevation and altimeter setting (QNH, in inHg or hPa), the tool SHALL compute station pressure via the ISA relation and pressure altitude PA = (T0/L)·(1 − (p/P0)^(R·L/g0)). This closed form applies below 36,089 ft; above it the tool SHALL invert the layered ISA model (isothermal tropopause and higher layers). It SHALL use the ISA-derived constants 145,442.16 ft and exponent 0.190263 (documented), and SHALL also show the rule of thumb PA ≈ elevation + (29.92 − setting) × 1,000 ft.

#### Scenario: 5,000 ft field, 29.80 inHg
- **WHEN** elevation = 5,000 ft and altimeter = 29.80 inHg
- **THEN** pressure altitude ≈ 5,108 ft (±1 ft), and the rule of thumb (5,120 ft) is shown with its difference

#### Scenario: Standard setting at sea level
- **WHEN** elevation = 0 ft and altimeter = 29.92126 inHg (1013.25 hPa)
- **THEN** pressure altitude = 0 ft (±0.1 ft)

### Requirement: Altimeter setting groups parsed
The altimeter input SHALL accept METAR-style groups (`A2980` → 29.80 inHg, `Q1009` → 1009 hPa) and plain values with units, and SHALL flag implausible values outside 26.00–32.00 inHg (880–1,085 hPa) as `SUSPECT_VALUE` without rejecting them.

#### Scenario: METAR group
- **WHEN** the altimeter input is `Q1009`
- **THEN** it is read as 1009 hPa

### Requirement: Density altitude, exact and approximate
The density altitude tool SHALL compute DA from the air density ratio (dry air by default): DA = (T0/L)·(1 − σ^(n/(1−n))), with n = R·L/g0. Humidity SHALL be an optional input. The tool SHALL also show the approximation DA ≈ PA + 118.8 × (OAT − ISA temperature at PA) ft (and the "120 ft per °C" rule), each labeled with its error.

#### Scenario: Hot, high field
- **WHEN** elevation = 5,000 ft, altimeter = 29.80 inHg, OAT = 30 °C, dry air
- **THEN** PA ≈ 5,108 ft, ISA temperature at PA ≈ 4.88 °C, DA ≈ 7,932 ft (±5 ft), and the approximation (≈ 8,093 ft) is shown with its +161 ft error

#### Scenario: Humidity raises DA
- **WHEN** the same case adds dew point 20 °C
- **THEN** DA is higher than the dry-air DA and the result states that humidity was included

### Requirement: ISA deviation and temperature at altitude
Tools SHALL compute ISA temperature at a pressure altitude (15 − 1.98 °C per 1,000 ft, capped at -56.5 °C above 36,089 ft) and ISA deviation from an OAT.

#### Scenario: Above the tropopause
- **WHEN** ISA temperature is requested at FL410
- **THEN** the result is -56.5 °C

### Requirement: Q-code conversions
Tools SHALL convert between QNH, QFE, and QNE/standard pressure given aerodrome elevation, and SHALL compute an altimeter setting from station pressure and elevation.

#### Scenario: QFE from QNH
- **WHEN** QNH = 1013 hPa and aerodrome elevation = 1,000 ft
- **THEN** QFE is computed via the ISA pressure-height relation and the altimeter reading on the ground with QFE set is 0 ft

### Requirement: Flight levels and transition
Tools SHALL convert between flight level and altitude for a given QNH, and SHALL show the regional transition-altitude convention as reference data (US: 18,000 ft MSL). The lowest usable flight level for a given QNH and transition altitude SHALL be computable.

#### Scenario: Lowest usable flight level
- **WHEN** QNH = 29.42 inHg in the US
- **THEN** the lowest usable flight level is FL185, and the FAA table is cited as reference

### Requirement: Cold-temperature altitude correction
The correction tool SHALL implement the ICAO Doc 8168 Vol II, 7th edition (2020) equation (as cited in Transport Canada AC 500-020 §4.8) ΔH = (−ΔT_std / L0) · ln(1 + L0·H_p / (T0 + L0·H_aerodrome)). Here L0 = −0.0019812 K/ft, ΔT_std is the aerodrome temperature minus ISA temperature at the aerodrome, and heights are relative to the altimeter-setting source. It SHALL NOT use the erroneous 2018 Vol III formulation. It SHALL also show the ICAO table method and the 4%-per-10 °C rule as labeled approximations. It SHALL support correcting multiple procedure altitudes at once (FAA "All Segments" or "Individual Segments" methods), and SHALL link to the live FAA Cold Temperature Airports list rather than hard-code it.

#### Scenario: Correction at -30 °C
- **WHEN** aerodrome elevation = 2,000 ft, aerodrome temperature = -30 °C, height above aerodrome = 1,500 ft
- **THEN** the correction ≈ +218 ft (to be added), and the 4% rule (≈ +246 ft) is shown as an approximation

#### Scenario: No correction when warmer than ISA
- **WHEN** the aerodrome temperature is at or above ISA
- **THEN** the correction is 0 and the tool explains that warm-temperature corrections are not applied

### Requirement: True altitude from indicated altitude
A tool SHALL estimate true altitude from indicated altitude, altimeter setting, and the actual temperature profile (via ISA deviation), showing the "high to low, look out below" effect.

#### Scenario: Flying into colder air
- **WHEN** the ISA deviation is -20 °C at indicated 8,000 ft with correct QNH
- **THEN** true altitude is below indicated altitude and the difference is reported

### Requirement: Altimeter visualization
Altimetry tools SHALL render a vertical diagram with elevation, pressure altitude, density altitude, and true altitude as labeled marks, plus a three-pointer altimeter gauge.

#### Scenario: Diagram marks
- **WHEN** density altitude is computed
- **THEN** the diagram shows field elevation, PA, and DA as labeled marks
