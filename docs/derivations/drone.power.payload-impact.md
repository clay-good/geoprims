<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# What a payload costs in flight time (`drone.power.payload-impact`)

## Method

The tool evaluates the same momentum-theory hover model twice: once at the aircraft's entered mass and once after adding the payload mass. Avionics power is included in both cases, while the payload's own electrical draw is included only when carried. Dividing the entered usable battery energy by each total power gives the two hover times. Their differences show the cost of the payload in watts, percent, and minutes.

## Equations

- `total disk area = rotors × π × rotor diameter² / 4`
- `lift power(m) = (m × g₀)^(3/2) / (√(2 × air density × total disk area) × FM × efficiency)`
- `power without = lift power(base mass) + avionics power`
- `power with = lift power(base mass + payload mass) + avionics power + payload power`
- `hover time = usable energy / power`
- `power increase percent = (power with − power without) / power without × 100`

## Symbols and units

Masses are converted to kilograms, rotor diameter to meters, usable energy to joules, and electrical loads to watts. `g₀` is standard gravity. Air density is in kg/m³ and comes from the shared ICAO troposphere model. `FM` is the dimensionless rotor figure of merit, and efficiency is the dimensionless combined motor and speed-controller efficiency. Hover times are returned in minutes by default.

## Domain

Aircraft mass, rotor diameter, and usable energy must be positive. Payload mass, payload power, and avionics power may be 0 but cannot be negative. Rotor count must be a whole number from 1 through 32. Figure of merit is from 0.4 through 0.8, and electrical efficiency is from 0.5 through 1. Altitude is limited to the ICAO troposphere, from -1,000 m through 11,000 m.

## Approximations

The model treats the rotors as ideal actuator disks and applies fixed loss factors. It does not know the motors' thrust ceiling, the aircraft's maximum takeoff mass, battery voltage sag, wind, ground effect, maneuvering, rotor interference, or the forward-flight drag of a bulky payload. The result is a hover estimate, not an aircraft approval. Check the aircraft manual and validate the estimate with a measured hover.

## Worked example

- sourcePublisher: MDPI Sensors
- sourceTitle: UAV-Based Servicing of IoT Nodes: Assessment of Ecological Impact
- sourceEdition: Volume 23, issue 4, article 2291 (2023)
- sourceLocator: Section 2.4.1, equation 6 and figure 4a
- independent: yes
- inputs: published payload-hover fit P = 10.6 W + 12.88(mg)^1.5, evaluated from 0.20 kg to 0.25 kg with 20 Wh; equivalent four-rotor disk area at the tool's default FM × efficiency
- outputs: 45.9787 W and 26.0990 min without the 50 g payload; 60.0432 W and 19.9856 min with it; 14.0645 W and 6.1134 min lost
- tolerance: 0.1% relative, covering the source's rounded fitted coefficients
- verifiedBy: golden vector v007 and `core/crates/gp-drone/tests/power_ops.rs`
- verifiedOn: 2026-10-13

Van Mulders et al. measured a small UAV hovering with calibrated payloads. Their fitted equation has the same mass-to-the-three-halves form as this tool and separately identifies 10.6 W of avionics power. The vector chooses the equivalent disk area whose default loss factors reproduce the published coefficient, then evaluates the paper's fit at two masses inside figure 4a's measured range.

## Differential tests

- `tools/vectors/gen_drone.py`: 25 vectors independently evaluate both hover-power states, endurance, unit conversions, electrical loads, atmosphere inputs, loss-factor limits, domain boundaries, and refusal cases
- vector v007 evaluates the independently published measured-data fit at base and payload masses

## Invariants

- `core/crates/gp-drone/tests/power_ops.rs` `payload_impact_matches_the_published_payload_hover_fit`: the tool reproduces both powers and both endurance values from the paper's fitted equation
- `core/crates/gp-drone/tests/power_ops.rs` `payload_lift_power_follows_mass_to_the_three_halves_power`: lift power changes by the exact momentum-theory mass ratio
- `core/crates/gp-drone/tests/power_ops.rs` `payload_electrical_draw_is_added_separately`: payload draw adds watt for watt without changing the no-payload case
- `core/crates/gp-drone/tests/power_ops.rs` `payload_time_scales_with_energy_and_zero_payload_has_zero_cost`: endurance scales linearly with energy, and a zero payload has zero cost
