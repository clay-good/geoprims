## Purpose

Estimates multirotor power draw, battery energy, and flight endurance from physics and user-supplied data. Losses, reserves, air density, and temperature are explicit, so estimates are conservative rather than optimistic.

## ADDED Requirements

### Requirement: Battery energy
Tools SHALL convert between capacity (mAh or Ah), nominal voltage (or cell count × nominal cell voltage, default 3.7 V for LiPo and 3.6 V for Li-ion, editable), and energy (Wh). They SHALL compute usable energy from a depth-of-discharge limit and a landing reserve, and SHALL compute current and C-rate for a power draw.

#### Scenario: Battery energy
- **WHEN** a 5,870 mAh, 15.4 V pack is entered
- **THEN** energy ≈ 90.4 Wh

#### Scenario: C-rate warning
- **WHEN** the computed current exceeds the user-entered maximum continuous C-rating
- **THEN** the result includes `C_RATE_EXCEEDED` with the ratio

### Requirement: Hover power from momentum theory
The hover-power tool SHALL compute ideal induced power P_ideal = T^1.5 / √(2ρA) with T = m·g0, rotor disk area A from rotor count and diameter (with a coaxial-overlap correction option), and air density ρ from the atmosphere tools (altitude and temperature, or density altitude). It SHALL require a figure of merit (default 0.6, range 0.4–0.8) and motor/ESC efficiency (default 0.85). It SHALL report electrical power P = P_ideal / (FM · η) plus a user-entered avionics/payload power, and SHALL never present P_ideal alone as the power draw.

#### Scenario: 1.4 kg quadcopter at sea level
- **WHEN** mass = 1.4 kg, 4 rotors of 9.4 in diameter, ISA sea level, FM = 0.6, η = 0.85
- **THEN** disk area ≈ 0.1791 m², ideal power ≈ 76.8 W, and electrical hover power ≈ 150.6 W

#### Scenario: Density altitude increases power
- **WHEN** the same aircraft hovers at 8,000 ft density altitude
- **THEN** hover power increases by the factor √(ρ0/ρ) ≈ 1.128 relative to sea level

### Requirement: Endurance and range
The endurance tool SHALL compute hover endurance = usable energy / total power, and a cruise estimate from user-entered cruise power (or a labeled generic multiplier of hover power). It SHALL apply a temperature derating (user-entered, with a labeled heuristic default of 0% at or above 20 °C, rising to 20% at 0 °C) and SHALL report endurance with and without reserve. Range SHALL be endurance × groundspeed, with wind effects via the aviation wind tools.

#### Scenario: Endurance with reserve
- **WHEN** the 150.6 W hover power is combined with the 90.4 Wh pack and 80% usable energy
- **THEN** hover endurance ≈ 28.8 min, and the result shows the reserve used and the derating applied

#### Scenario: Cold battery
- **WHEN** battery temperature is 0 °C with the default heuristic
- **THEN** usable energy is reduced by 20%, labeled `HEURISTIC_DERATING`

### Requirement: Payload impact
A tool SHALL compute the change in hover power and endurance from adding payload mass and payload electrical power, and the maximum payload for a target endurance.

#### Scenario: Maximum payload
- **WHEN** a target hover endurance of 20 min is requested
- **THEN** the tool returns the maximum additional mass that still meets it, or `NO_SOLUTION` if even zero payload cannot

### Requirement: Return-to-home energy budget
Given the current position, home position, groundspeed capability, wind, and remaining energy, a tool SHALL compute the energy and time required to return (into-wind groundspeed via the wind triangle), the margin remaining, and the maximum outbound distance for a round trip that keeps the reserve.

#### Scenario: Headwind return
- **WHEN** the return leg faces a 10 m/s headwind with 15 m/s airspeed
- **THEN** the return groundspeed is 5 m/s, and the required energy and margin are reported

#### Scenario: Cannot return
- **WHEN** the headwind equals or exceeds the airspeed
- **THEN** the result is `NO_SOLUTION` with warning `CANNOT_RETURN_INTO_WIND`

### Requirement: Peukert and other battery models are labeled
If a Peukert-style correction is offered, it SHALL be labeled as a weak model for lithium chemistries and SHALL be off by default.

#### Scenario: Peukert off by default
- **WHEN** a user opens the endurance tool
- **THEN** Peukert correction is off and its toggle explains its limits
