## Purpose

Converts between indicated, calibrated, equivalent, and true airspeed and Mach number with the exact compressible-flow relations, so that the numbers pilots and engineers rely on are right at every altitude and speed.

## ADDED Requirements

### Requirement: IAS ↔ CAS requires a calibration table
IAS ↔ CAS conversion SHALL use a user-supplied airspeed calibration table (from the POH/AFM, optionally per flap setting) with linear interpolation. It SHALL refuse to extrapolate beyond the table (`OUT_OF_DOMAIN`), and SHALL never apply a built-in default correction. If no table is supplied, the tool SHALL state that IAS is being treated as CAS and warn `CALIBRATION_ASSUMED`.

#### Scenario: No calibration table
- **WHEN** a user converts IAS to TAS without a calibration table
- **THEN** the result treats IAS as CAS and includes `CALIBRATION_ASSUMED`

#### Scenario: Beyond table
- **WHEN** IAS 180 kt is converted with a table ending at 160 kt
- **THEN** the result is `OUT_OF_DOMAIN` naming the table range

### Requirement: CAS ↔ TAS ↔ Mach via impact pressure
Conversions SHALL follow the exact path CAS → impact pressure qc (sea-level relation) → Mach (using static pressure at pressure altitude) → TAS (using actual static air temperature), and the reverse. Subsonic flow SHALL use the isentropic relation. For supersonic flow the tool SHALL use the Rayleigh pitot formula, iterated to 1e-12 relative. The result SHALL display the intermediate qc, Mach, and speed of sound.

#### Scenario: CAS 250 kt at FL100
- **WHEN** CAS = 250 kt, pressure altitude = 10,000 ft, OAT = -5 °C
- **THEN** Mach ≈ 0.4523, TAS ≈ 288.6 kt, and EAS ≈ 248.1 kt (±0.1 kt)

#### Scenario: High altitude
- **WHEN** CAS = 300 kt, pressure altitude = 35,000 ft, OAT = -54.3 °C
- **THEN** Mach ≈ 0.8736 (±0.0005) and TAS ≈ 503.6 kt (±0.1 kt)

#### Scenario: Supersonic branch
- **WHEN** a CAS corresponds to Mach > 1 at the given altitude
- **THEN** the tool switches to the Rayleigh pitot formula and states the branch used

### Requirement: OAT, not ISA temperature, for TAS
TAS tools SHALL require an actual temperature input and SHALL default it to the ISA temperature only when the user explicitly selects "standard temperature," in which case warning `ISA_TEMPERATURE_ASSUMED` SHALL be attached.

#### Scenario: Standard temperature selected
- **WHEN** a user selects standard temperature for a TAS calculation
- **THEN** the result includes `ISA_TEMPERATURE_ASSUMED`

### Requirement: Total and static air temperature
Tools SHALL convert between total (indicated) air temperature and static air temperature: SAT = TAT / (1 + 0.2 · r · M²), with probe recovery factor r (default 1.0, user-adjustable). They SHALL also compute ram rise.

#### Scenario: Ram rise at Mach 0.8
- **WHEN** TAT = -20 °C at Mach 0.8 with r = 1.0
- **THEN** SAT ≈ -48.73 °C and ram rise ≈ 28.73 K

### Requirement: EAS and dynamic pressure
Tools SHALL compute equivalent airspeed (EAS = TAS·√σ), dynamic pressure q = ½ρV², and the compressibility correction (CAS − EAS), which SHALL never be negative.

#### Scenario: Compressibility correction sign
- **WHEN** any subsonic CAS and altitude are converted
- **THEN** CAS − EAS ≥ 0

### Requirement: Rules of thumb shown and labeled
Airspeed results SHALL show the common approximation (TAS ≈ CAS + 2% per 1,000 ft) alongside the exact value, labeled "rule of thumb", with its error.

#### Scenario: Approximation error displayed
- **WHEN** the FL100 scenario is computed
- **THEN** the rule-of-thumb TAS (300 kt) is displayed with its difference from the exact 288.6 kt

### Requirement: Airspeed tape visualization
Airspeed tools SHALL render a gauge showing IAS/CAS/EAS/TAS/Mach together, with user-entered V-speeds (e.g. VS0, VS1, VFE, VNO, VNE, MMO) as colored arcs whose meanings are also conveyed by labels (not color alone).

#### Scenario: V-speed arcs
- **WHEN** a user enters VS0, VS1, VFE, VNO, and VNE
- **THEN** the gauge draws the white, green, and yellow arcs and the red line, each labeled
