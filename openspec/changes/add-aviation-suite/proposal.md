## Why

Pilots, dispatchers, flight-test engineers, and UAM teams run the same calculations every day: pressure and density altitude, CAS to TAS to Mach, wind correction, crosswind limits, cold-temperature altitude corrections, turn radius, top of descent, fuel, and weight and balance. Today they use a physical E6B, a paid app ($8.99 and up), or ad-laden web calculators that silently mix rules of thumb with exact formulas. Common errors include:

- using ISA temperature instead of OAT for TAS
- using P0 instead of static pressure for Mach
- using the flawed 2018 cold-temperature equation
- mixing magnetic and true wind

geoprims provides exact, sourced atmospheric and flight-mechanics math, shows the rule-of-thumb value next to the exact one, and draws every result as an instrument or vector diagram.

Depends on: `establish-platform-foundation`, `add-geodesy-suite` (magnetic variation), `add-navigation-and-geometry` (routes, turns).

## What Changes

Adds the `aviation` domain: about 83 operations and 103 tool ids (inventory in `design.md`).

- **Atmosphere:** ICAO Standard Atmosphere −5 to 80 km with geopotential/geometric altitude handling; US Standard Atmosphere 1976 to 86 km; non-standard days (ISA deviation); speed of sound; viscosity; humidity and dew point; cloud-base estimate.
- **Airspeed:** IAS↔CAS (user calibration table), CAS↔EAS↔TAS↔Mach (subsonic and supersonic pitot), impact pressure, TAT↔SAT with probe recovery factor.
- **Altimetry:** pressure altitude from QNH, QFE/QNH/QNE conversions, density altitude (exact and rule of thumb), flight levels, altimeter setting from station pressure, ICAO/FAA cold-temperature corrections, true altitude.
- **Wind and navigation (E6B):** every wind-triangle variant, runway wind components and limits, best-runway selection, heading chain (true → magnetic → compass with deviation card), and the 1-in-60 off-course correction.
- **Flight performance:** turn radius, rate, bank, load factor, and stall speed in a turn; climb and descent gradients; top of descent; glide range with wind; pivotal altitude.
- **Fuel and loading:** fuel burn, endurance, reserves (regulatory reference values dated with source), weight and balance with CG envelope check, and interpolation of user-supplied POH/AFM performance tables.

## Capabilities

### New Capabilities

- `aviation/atmosphere`: Standard and non-standard atmosphere models and air properties.
- `aviation/airspeed`: Airspeed conversions and pitot-static relations.
- `aviation/altimetry`: Pressure, density, and true altitude, and altimeter settings.
- `aviation/wind-and-navigation`: Wind triangles, runway components, heading chains, and E6B navigation functions.
- `aviation/flight-performance`: Turn, climb, descent, and glide performance math.
- `aviation/fuel-and-loading`: Fuel planning, weight and balance, and POH table interpolation.

### Modified Capabilities

None.

## Non-goals

- Live weather (METAR/TAF), NOTAMs, airspace, or airport databases. Users supply values.
- Aircraft-specific performance models. Users supply POH/AFM data.
- Certified or approved use. Every tool carries the not-for-navigation notice.
- Supersonic aerodynamics beyond pitot relations. Weapons and ballistics are excluded.

## Impact

- `core/gp-aviation` crate.
- Reference data: ICAO Doc 7488/3 tables, US Standard Atmosphere 1976 tables, ICAO Doc 8168 Vol II, 7th edition (2020) cold-temperature method as cited in Transport Canada AC 500-020 §4.8, AIM chapter 7-3, and 14 CFR 91.151/91.167 (dated references).
- Vector-diagram and instrument (gauge) canvas layers are exercised heavily.
