## Purpose

Covers the fuel, weight-and-balance, and aircraft-data interpolation calculations of preflight planning, using the user's own aircraft data and dated regulatory reference values.

## ADDED Requirements

### Requirement: Fuel planning
Tools SHALL compute fuel required (burn rate × time plus taxi, climb increments, and reserve), endurance from usable fuel, and fuel weight from volume (with fuel type density presets labeled nominal, or user density), per leg and in total.

#### Scenario: Fuel weight
- **WHEN** 40 US gal of 100LL is converted with the nominal preset (6.0 lb/gal)
- **THEN** the result is 240 lb, labeled nominal

### Requirement: Reserve requirements as dated reference data
Fuel reserve presets SHALL cite their regulation and "rules as of" date and SHALL depend on an aircraft-category input (airplane or rotorcraft): for rotorcraft, 14 CFR 91.151 requires 20 min VFR and 91.167 requires 30 min after the alternate. For airplanes they SHALL include at minimum 14 CFR 91.151 (VFR day 30 min, VFR night 45 min) and 14 CFR 91.167 (IFR: to the destination, then the alternate, then 45 min at normal cruise). The tool SHALL show that operators may have stricter rules, and SHALL allow a custom reserve.

#### Scenario: VFR night reserve
- **WHEN** a user selects "VFR night (14 CFR 91.151)"
- **THEN** a 45-minute reserve at the entered cruise burn is added, and the citation with its date is shown

#### Scenario: Rotorcraft reserve
- **WHEN** a user selects aircraft category rotorcraft and "VFR (14 CFR 91.151)"
- **THEN** a 20-minute reserve is applied and cited

### Requirement: Weight and balance
Given station weights and arms (user-defined stations: empty weight, seats, baggage, fuel), the tool SHALL compute total weight, total moment, and CG, and optionally CG in % MAC (from LEMAC and MAC length). It SHALL check the result against a user-entered CG envelope polygon (weight versus CG), for the takeoff and landing states, with fuel burn between them.

#### Scenario: CG computation
- **WHEN** stations are empty 1,500 lb @ 85 in, front 340 lb @ 90 in, rear 170 lb @ 118 in, fuel 240 lb @ 48 in
- **THEN** total weight = 2,250 lb and CG ≈ 84.30 in

#### Scenario: Out of envelope
- **WHEN** the takeoff point lies outside the user's envelope
- **THEN** the result reports `OUTSIDE_CG_ENVELOPE` with the distance to the nearest envelope edge in inches and pounds, and the diagram marks the point

#### Scenario: Landing CG shift
- **WHEN** fuel burn is entered
- **THEN** both takeoff and landing CG points are computed and drawn, with the path between them

### Requirement: POH/AFM table interpolation
A generic tool SHALL interpolate user-entered performance tables of one, two, or three independent variables (e.g. takeoff distance vs pressure altitude, temperature, and weight). It SHALL use linear or bilinear/trilinear interpolation, SHALL refuse extrapolation (`OUT_OF_DOMAIN`, naming the violated axis), and SHALL apply user-entered correction factors (e.g. headwind, slope, surface) as separate, labeled steps.

#### Scenario: Bilinear interpolation
- **WHEN** a takeoff-distance table indexed by pressure altitude (0/2,000/4,000 ft) and temperature (0/10/20/30 °C) is queried at 3,000 ft and 25 °C
- **THEN** the result is the bilinear interpolation of the four surrounding cells, and the cells used are highlighted

#### Scenario: Extrapolation refused
- **WHEN** the table is queried at 5,000 ft
- **THEN** the result is `OUT_OF_DOMAIN` naming the pressure-altitude axis and its range

### Requirement: Aircraft profiles saved locally
Users SHALL be able to save aircraft profiles (stations, envelope, calibration table, performance tables, fuel burn) in local storage, and export and import them as JSON files. Nothing SHALL be uploaded.

#### Scenario: Profile export
- **WHEN** a user exports an aircraft profile
- **THEN** a JSON file downloads containing the profile, and re-importing it restores all tables
