## Purpose

Computes the properties of the standard and non-standard atmosphere (temperature, pressure, density, speed of sound, viscosity, and humidity effects) that every airspeed, altimetry, performance, and drone-endurance calculation depends on.

## ADDED Requirements

### Requirement: ICAO Standard Atmosphere
The ISA tool SHALL compute temperature, pressure, density, their ratios to sea level (θ, δ, σ), speed of sound, dynamic and kinematic viscosity (Sutherland's law), and gravitational acceleration at altitude. It SHALL follow ICAO Doc 7488/3 and ISO 2533:1975 from -5 km to 80 km geopotential altitude, using the constants P0 = 101,325 Pa, T0 = 288.15 K, g0 = 9.80665 m/s², R = 287.05287 J/(kg·K), γ = 1.4, and r0 = 6,356,766 m, and the layer table (0, 11, 20, 32, 47, 51, 71, 80 km geopotential). Inputs SHALL be geometric or geopotential altitude (declared), converted by H = r0·z/(r0 + z).

#### Scenario: 10,000 ft
- **WHEN** ISA properties are requested at 10,000 ft geopotential
- **THEN** T = 268.338 K (-4.812 °C), P ≈ 69,681.6 Pa (696.82 hPa), ρ ≈ 0.904637 kg/m³ (±0.01%)

#### Scenario: Tropopause
- **WHEN** ISA properties are requested at 11,000 m geopotential (36,089 ft)
- **THEN** T = 216.65 K and P ≈ 22,632 Pa, and the result names the layer "tropopause (isothermal)"

#### Scenario: Geometric vs geopotential
- **WHEN** 11,000 m is entered as geometric altitude
- **THEN** it is converted to ≈ 10,981 m geopotential before evaluation, and the result shows both values

#### Scenario: Above the model
- **WHEN** 90 km is requested with the ICAO model
- **THEN** the result is `OUT_OF_DOMAIN` with a hint that US Standard Atmosphere 1976 extends to 86 km in this tool

### Requirement: US Standard Atmosphere 1976
A selectable model SHALL implement US Standard Atmosphere 1976 to 86 km geometric altitude. It SHALL match the published tables within table rounding, and SHALL state that it is identical to ISA below 32 km.

#### Scenario: Table agreement
- **WHEN** the US76 model is evaluated at every 1 km tabulated altitude to 86 km
- **THEN** results match the published table values within their printed precision

### Requirement: Non-standard atmospheres
The atmosphere tool SHALL accept an ISA temperature deviation (a temperature difference, per the units spec) or an actual temperature at altitude. It SHALL compute the resulting density and density altitude while keeping the pressure-altitude relationship standard, and SHALL state this assumption.

#### Scenario: ISA+20
- **WHEN** ISA +20 °C is requested at pressure altitude 5,000 ft
- **THEN** pressure equals the ISA pressure at 5,000 ft, temperature is ISA +20 K, and density is reduced accordingly

### Requirement: Humidity and moist-air effects
Tools SHALL compute relative humidity, dew point, and vapor pressure from each other (Magnus-type formula with coefficients cited and stated accuracy), virtual temperature, and density including humidity. Density-altitude results SHALL state whether humidity was included.

#### Scenario: Humid density
- **WHEN** density is computed at 30 °C, 1,000 hPa, dew point 24 °C
- **THEN** moist density is lower than dry density at the same T and P, and both are reported

### Requirement: Cloud base and freezing level estimates
Tools SHALL estimate convective cloud base from the temperature-dew point spread (about 400 ft per °C, labeled as an approximation, with the more exact lifting-condensation-level formula alongside) and the freezing level from surface temperature and a lapse rate (default standard, user-adjustable).

#### Scenario: Cloud base approximation
- **WHEN** surface temperature is 25 °C and dew point 15 °C
- **THEN** the estimated cloud base is about 4,000 ft AGL (rule of thumb), with the LCL formula result shown alongside

### Requirement: Atmosphere profile visualization
Atmosphere tools SHALL render a profile chart of T, P, ρ, and speed of sound versus altitude, with layer boundaries marked and the queried altitude highlighted.

#### Scenario: Profile chart
- **WHEN** a user queries 30,000 ft
- **THEN** the profile chart highlights 30,000 ft and marks the tropopause at 36,089 ft
