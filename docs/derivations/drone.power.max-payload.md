<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Maximum payload for a flight time (`drone.power.max-payload`)

## Method

The tool turns the entered usable energy and target hover time into an electrical-power budget. It subtracts the avionics and payload electrical draw, then solves the momentum-theory hover equation backward for the greatest total mass that fits the remaining lift-power budget. Subtracting the aircraft's entered mass gives the maximum payload. The same model also computes the aircraft's no-payload hover time.

## Equations

- `total disk area = rotors × π × rotor diameter² / 4`
- `available electrical power = usable energy / target time`
- `available lift power = available electrical power − avionics power`
- `maximum mass = ((available lift power × FM × efficiency × √(2 × air density × total disk area))^(2/3)) / g₀`
- `maximum payload = maximum mass − aircraft mass`
- `base hover time = usable energy / (lift power(aircraft mass) + avionics power)`

## Symbols and units

Mass is converted to kilograms, rotor diameter to meters, usable energy to joules, target time to seconds, and avionics power to watts. `g₀` is standard gravity. Air density is in kg/m³ and comes from the shared ICAO troposphere model. `FM` is the dimensionless rotor figure of merit, and efficiency is the dimensionless combined motor and speed-controller efficiency.

## Domain

Aircraft mass, rotor diameter, usable energy, and target time must be positive. Avionics and payload power may be 0 but cannot be negative. Rotor count must be a whole number from 1 through 32. Figure of merit is from 0.4 through 0.8, and electrical efficiency is from 0.5 through 1. Altitude is limited to the ICAO troposphere, from -1,000 m through 11,000 m. The tool returns `NO_SOLUTION` when the entered aircraft cannot meet the target time before carrying any payload.

## Approximations

The model treats the rotors as ideal actuator disks and applies fixed loss factors. It does not know the motors' thrust ceiling, the frame's structural limit, the manufacturer's maximum takeoff mass, battery voltage sag, wind, ground effect, maneuvering, rotor interference, or the forward-flight drag of a bulky payload. The result is a power-limited hover estimate, not permission to carry the load. The aircraft manual and a measured hover test govern.

## Worked example

- sourcePublisher: MDPI Sensors
- sourceTitle: UAV-Based Servicing of IoT Nodes: Assessment of Ecological Impact
- sourceEdition: Volume 23, issue 4, article 2291 (2023)
- sourceLocator: Section 2.4.1, equation 6 and figure 4a
- independent: yes
- inputs: published payload-hover fit `P = 10.6 W + 12.88(mg)^1.5`; 0.20 kg aircraft, 20 Wh, and the 19.9856 min duration that the fit predicts at 0.25 kg; equivalent four-rotor disk area at the tool's default FM × efficiency
- outputs: 0.2500 kg maximum mass, 0.0500 kg maximum payload, and 26.0990 min base hover time
- tolerance: 0.1% relative, covering the source's rounded fitted coefficients
- verifiedBy: golden vector v006 and `core/crates/gp-drone/tests/power_ops.rs`
- verifiedOn: 2026-10-13

Van Mulders et al. measured a small UAV hovering with calibrated payloads and fitted its power to the same mass-to-the-three-halves form this tool inverts. The vector starts with the paper's 0.25 kg fitted hover power, converts that power to a duration on 20 Wh, and asks the tool for the payload that meets that duration. The answer recovers the 0.25 kg total mass and 50 g payload.

## Differential tests

- `tools/vectors/gen_drone.py`: 25 vectors independently solve the inverse equation across equivalent units, avionics draw, atmosphere inputs, loss-factor limits, rotor-count and altitude boundaries, large aircraft, and refusal cases
- vector v006 inverts the independently published measured-data fit

## Invariants

- `core/crates/gp-drone/tests/power_ops.rs` `maximum_payload_inverts_the_published_payload_hover_fit`: the tool recovers the paper's 0.25 kg fitted mass and 50 g payload
- `core/crates/gp-drone/tests/power_ops.rs` `maximum_mass_scales_with_available_power_to_two_thirds`: doubling the available power multiplies maximum mass by `2^(2/3)`
- `core/crates/gp-drone/tests/power_ops.rs` `maximum_payload_is_unchanged_by_equivalent_units_and_energy_time_scaling`: equivalent units and equal energy-per-time ratios preserve the mass result
- `core/crates/gp-drone/tests/power_ops.rs` `lower_air_density_reduces_maximum_payload`: an 8,000 ft density altitude reduces the supported payload
- `core/crates/gp-drone/tests/power_ops.rs` `maximum_payload`: hover power at the returned mass exactly consumes the entered energy over the target time, and an infeasible target returns `NO_SOLUTION`
