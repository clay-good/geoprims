<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Multirotor hover power (`drone.power.hover-power`)

## Method

The tool treats every rotor as an actuator disk. In hover, total thrust equals aircraft weight. Ideal induced power follows momentum theory; dividing by the rotor figure of merit and electrical efficiency turns that lower bound into an electrical-power estimate. A fixed avionics or payload draw is added last. Air density comes from the shared ICAO troposphere model, using either pressure altitude and temperature or density altitude.

For stacked coaxial pairs, only induced power receives the entered interference factor. The profile-loss share represented by the figure of merit is not multiplied again. An odd rotor count cannot form pairs and is refused.

## Equations

- Total disk area: `A = N π D² / 4`
- Hover thrust: `T = m g₀`
- Ideal induced power: `Pᵢ = T^(3/2) / √(2 ρ A)`
- Separate rotors: `P = Pᵢ / (FM η) + Pₐ`
- Coaxial pairs: `P = Pᵢ (κ + 1/FM − 1) / η + Pₐ`
- Disk loading: `T / A`
- Density power factor relative to sea level: `√(ρ₀ / ρ)`

## Symbols and units

`m` is takeoff mass in kilograms, `g₀` is standard gravity in m/s², `N` is rotor count, `D` is rotor diameter in meters, `A` is total disk area in m², `ρ` is air density in kg/m³, `FM` is figure of merit, `η` is motor and speed-controller efficiency, `κ` is the coaxial induced-power factor, and `Pₐ` is avionics and payload power in watts. Power outputs are watts and disk loading is pascals.

## Domain

Mass and diameter must be positive, rotor count must be a whole number from 1 through 32, figure of merit is 0.4 through 0.8, and electrical efficiency is 0.5 through 1. The atmosphere input is limited to the troposphere, from -1,000 m through 11,000 m. Coaxial mode requires an even rotor count; an interference factor is accepted only in that mode.

## Approximations

Momentum theory assumes steady, uniform, inviscid flow through ideal actuator disks. The estimate does not model blade geometry, frame download, ground effect, wind, maneuvering, or forward flight. The default figure of merit and electrical efficiency are engineering starting points, not measured properties of the reader's aircraft. Coaxial interference uses the Leishman and Syal fully developed-slipstream factor by default; real spacing and thrust sharing change it.

## Worked example

- sourcePublisher: Embry-Riddle Aeronautical University
- sourceTitle: Introduction to Aerospace Flight Vehicles
- sourceEdition: 2026 webbook edition
- sourceLocator: Chapter 67, hover-power worked example
- independent: yes
- inputs: 45,000 lb aircraft, two 38 ft rotors, sea-level standard density, figure of merit 0.75, and no transmission loss in the compared result
- outputs: 3,940.4 kW ideal hover power and 5,253.8 kW after the figure of merit
- tolerance: 0.05% relative, covering the source's rounded intermediate values
- verifiedBy: golden vector v013, run by the core on every build
- verifiedOn: 2026-10-13

The source works each rotor at half the weight, prints 1,970.2 kW ideal power per rotor, and doubles that result. Geoprims computes the aircraft as one equivalent total disk, which is algebraically the same for two equal rotors, and returns 3,941.46 kW ideal and 5,255.28 kW at `FM = 0.75`. Both differ from the printed values by less than 0.03%. The source then applies a separate 5% transmission loss; that final 5,515.7 kW figure is not compared because the tool's efficiency input was set to 1 for this vector.

## Differential tests

- `tools/vectors/gen_drone.py` generates 19 input cases from an independent implementation of the momentum and atmosphere equations, plus the published Embry-Riddle example. The 20 vectors cover 0.5 kg through 45,000 lb, 2 through 12 rotors, metric and US rotor sizes, pressure and density altitude, temperature, avionics draw, both loss factors, coaxial interference, and both coaxial input errors.

## Invariants

- `core/crates/gp-drone/tests/power_ops.rs` `hover_power_scaling_invariants` checks that power follows mass to the 3/2 power and falls with the square root of disk area.
- `core/crates/gp-drone/tests/power_ops.rs` `density_altitude_increases_power` checks the exact density factor and that lower density raises power.
- `core/crates/gp-drone/tests/power_ops.rs` `calibration_round_trips_through_hover_power` feeds a measured calibration back into this tool and recovers the measured power.
