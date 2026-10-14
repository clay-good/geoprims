<!-- Derivation note (trust/correctness-program layer A). -->
# Usable battery energy (`drone.power.usable-energy`)

## Method

A pack's rated energy is not all available to a flight plan. A discharge limit protects the cells, a cold pack gives less, and a share is held back to land with. The operation applies those in the same order and with the same function as the energy budget inside `drone.power.endurance`.

## Equations

- Usable = E × usable share × (1 − derating)
- Energy to fly on = usable × (1 − reserve)
- Reserve kept = usable × reserve
- Cold derating from a battery temperature T: 0 at or above 20 °C, (20 − T) × 1% below it, capped at 50%

## Symbols and units

E rated energy in watt-hours; usable share, reserve, and derating in percent; battery temperature in °C, °F, or K.

## Domain

Positive energy. Shares from 0 to 100%. An entered derating takes precedence over a battery temperature.

## Approximations

The cold derating is a labeled rule of thumb and raises HEURISTIC_DERATING when it applies. Voltage sag is not modeled.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-drone-suite specification
- sourceEdition: 2026
- sourceLocator: drone/power, requirement "Endurance" (usable energy, reserve, and cold derating); checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: 99 Wh, 85% usable, 25% reserve
- outputs: 84.15 Wh usable, 21.04 Wh reserve, 63.11 Wh to fly on
- tolerance: 1e-9 Wh
- verifiedBy: golden vector v001 and the `focused_power_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `usable_energy_vectors`: 18 random budgets over nine pack sizes with usable shares, reserves, entered deratings, and battery temperatures from −25 to 30 °C.
- `core/vectors/drone.power.usable-energy.jsonl`: those vectors, two hand-set cases, and two refusal cases.
- The integration suite requires the same usable and reserve energy as `drone.power.endurance`.

## Invariants

- Energy to fly on + reserve kept = usable energy.
- With no shares entered, every figure is the rated energy.
- A colder pack never has more usable energy.
