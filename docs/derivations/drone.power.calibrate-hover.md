<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Calibrate hover power from a test flight (`drone.power.calibrate-hover`)

## Method

The tool turns energy used during a steady hover into measured electrical power. After subtracting separately measured avionics and payload draw, it compares the remaining lift power with the ideal actuator-disk power for the aircraft's mass, rotor area, and air density. That ratio is the combined figure of merit times motor and speed-controller efficiency. Dividing it by the entered electrical efficiency gives the rotor figure of merit used by the hover-power tool.

## Equations

- `measured power = energy used / hover time`
- `total disk area = rotors × π × rotor diameter² / 4`
- `ideal power = (mass × g₀)^(3/2) / √(2 × air density × total disk area)`
- `FM × efficiency = ideal power / (measured power − avionics power)`
- `FM = (FM × efficiency) / efficiency`

## Symbols and units

Energy is converted to joules, time to seconds, mass to kilograms, rotor diameter to meters, and power to watts before evaluation. `g₀` is standard gravity. Air density is in kg/m³ and comes from the shared ICAO troposphere calculation. `FM` is the dimensionless rotor figure of merit, and efficiency is the dimensionless combined motor and speed-controller efficiency.

## Domain

Mass, rotor diameter, energy used, and hover time must be positive. The rotor count must be a whole number from 1 through 32. Avionics power must be at least 0 and below measured power. Electrical efficiency is from 0.5 through 1. Altitude is limited to the ICAO troposphere, from -1,000 m through 11,000 m. A result at or above an ideal combined efficiency of 1 is refused because it would beat ideal momentum theory.

## Approximations

The aircraft is represented by ideal actuator disks in steady hover. The calculation does not model wind, ground effect, maneuvering, frame download, rotor interference, changing battery voltage, or measurement error. Energy must cover the steady-hover interval alone. Separating rotor figure of merit from electrical efficiency depends on the efficiency entered by the user; the directly observed quantity is their product.

## Worked example

- sourcePublisher: Embry-Riddle Aeronautical University
- sourceTitle: Introduction to Aerospace Flight Vehicles
- sourceEdition: 2026 webbook edition
- sourceLocator: Chapter 67, hover-power worked example
- independent: yes
- inputs: 45,000 lb aircraft, two 38 ft rotors, 5,253.8 kW measured power represented as 5,253.8 kWh over 1 h, sea-level standard density, and electrical efficiency 1
- outputs: figure of merit 0.75 and combined figure of merit times efficiency 0.75
- tolerance: 0.05% relative, covering the source's rounded intermediate power values
- verifiedBy: golden vector v007 and `core/crates/gp-drone/tests/power_ops.rs`
- verifiedOn: 2026-10-13

Embry-Riddle calculates 3,940.4 kW of ideal power and 5,253.8 kW after dividing by a figure of merit of 0.75, before a separate transmission loss. The calibration tool runs that relation backward. Its exact constants give a combined factor within 0.03% of the published 0.75.

## Differential tests

- `tools/vectors/gen_drone.py`: 25 vectors independently evaluate the momentum, atmosphere, energy, and time relations across metric and US units, 1 through 32 rotors, both altitude forms, temperature, avionics draw, efficiency limits, domain boundaries, and refusal cases
- vector v007 inverts the independently published Embry-Riddle example within its printed precision

## Invariants

- `core/crates/gp-drone/tests/power_ops.rs` `calibration_round_trips_through_hover_power`: feeding the calibrated figure of merit back into hover power recovers the measured power
- `core/crates/gp-drone/tests/power_ops.rs` `calibration_recovers_hover_parameters_across_aircraft`: calibration recovers the starting figure of merit and efficiency product for varied aircraft and atmospheres
- `core/crates/gp-drone/tests/power_ops.rs` `calibration_units_do_not_change_the_result`: equivalent metric, US, energy, and time units leave the result unchanged
- `core/crates/gp-drone/tests/power_ops.rs` `calibration_subtracts_non_lift_power_before_back_solving`: avionics draw is removed before the lift-power ratio is formed
