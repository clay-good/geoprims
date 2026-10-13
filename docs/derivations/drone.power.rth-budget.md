<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Return-to-home energy budget (`drone.power.rth-budget`)

## Method

The tool treats the entered wind as steady and directly along the outbound and return track. A headwind slows the return groundspeed and helps the outbound leg by the same amount. Return time is distance divided by return groundspeed, and return energy is constant cruise power multiplied by that time. The margin subtracts both the requested reserve and return energy from the remaining energy. The farthest round trip solves the outbound-plus-return time equation for the equal distance of each leg.

## Equations

- `return groundspeed = airspeed − headwind`
- `outbound groundspeed = airspeed + headwind`
- `return time = distance home / return groundspeed`
- `return energy = cruise power × return time`
- `margin = remaining energy − reserve energy − return energy`
- `maximum one-way round-trip distance = (remaining energy − reserve energy) / (cruise power × (1 / outbound groundspeed + 1 / return groundspeed))`

## Symbols and units

Distance is converted to meters, airspeed and headwind to meters per second, power to watts, and remaining and reserve energy to joules. Time is returned in minutes, energy in watt-hours, and maximum one-way round-trip distance in kilometers by default. A negative headwind is a tailwind on the return leg.

## Domain

Distance, airspeed, cruise power, and remaining energy must be positive. Reserve energy may be 0 but cannot be negative. The absolute wind speed must remain below the airspeed, because otherwise one leg has no positive groundspeed. The tool may return a negative margin when the aircraft lacks the entered reserve or return energy; that is an answer, not a calculation failure.

## Approximations

The model uses one steady along-track wind and constant airspeed and power. It omits crosswind correction, gusts, wind gradients, climbs, descents, landing energy, maneuvering, cold-pack derating, aging, and voltage sag. The remaining energy must already reflect the battery state. A positive margin is an arithmetic reserve under these assumptions, not a claim that the flight is safe.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Fostering Aviation Activities
- sourceEdition: Junior High Level aviation curriculum guide (1995)
- sourceLocator: Air Speed Indicator, pages 23-25
- independent: yes
- inputs: 134.4 mph airspeed, 30 mph headwind, 10 mi home, 200 W cruise power, 100 Wh remaining, and 20 Wh reserve
- outputs: 104.4 mph (46.670976 m/s) return groundspeed, 5.7471 min return time, 19.1571 Wh return energy, 60.8429 Wh margin, and 41.1038 km maximum one-way round-trip distance
- tolerance: groundspeed within 0.000001 m/s; derived fields within 1 part in 1,000,000,000
- verifiedBy: golden vector v006 and `core/crates/gp-drone/tests/power_ops.rs`
- verifiedOn: 2026-10-13

The FAA example prints the wind calculation directly: 134.4 mph true airspeed minus a 30 mph headwind gives 104.4 mph groundspeed. The vector uses that published speed for the return leg, then applies the tool's distance, power, energy, and reserve arithmetic.

## Differential tests

- `tools/vectors/gen_drone.py`: 25 vectors independently evaluate groundspeed, time, energy, margin, and round-trip distance across headwind, tailwind, calm air, equivalent units, boundary winds, energy deficits, and invalid inputs
- vector v006 starts from the FAA's independently published headwind result

## Invariants

- `core/crates/gp-drone/tests/power_ops.rs` `rth_budget_matches_the_faa_headwind_example`: return groundspeed matches the FAA's printed 104.4 mph result, and time and energy follow from it
- `core/crates/gp-drone/tests/power_ops.rs` `rth_energy_scales_with_distance_and_power`: doubling either distance or power doubles return energy
- `core/crates/gp-drone/tests/power_ops.rs` `rth_round_trip_distance_is_symmetric_when_wind_reverses`: reversing the steady wind swaps the two legs without changing the full round-trip reach
- `core/crates/gp-drone/tests/power_ops.rs` `rth_budget_preserves_equivalent_units_and_refuses_negative_reserve`: equivalent SI inputs preserve every result, and a negative reserve is refused
- `core/crates/gp-drone/tests/power_ops.rs` `headwind_return_and_cannot_return`: the original scenario gives 5 m/s home and 15 Wh used, while a wind equal to airspeed returns `NO_SOLUTION`
