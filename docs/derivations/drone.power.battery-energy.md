<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Battery energy, mAh to Wh (`drone.power.battery-energy`)

## Method

Energy is capacity times nominal voltage. The voltage is entered, or comes from a cell count times the chemistry's nominal cell voltage, and the tool says it used a nominal value. Usable energy is the energy between the depth-of-discharge limit and the reserve. With a steady power draw, the current and the C-rate follow, and a draw above the pack's continuous rating is flagged.

## Equations

- E (Wh) = capacity (Ah) × V = capacity (mAh) / 1,000 × V.
- V = n × V_cell, with V_cell = 3.7 V (LiPo) or 3.6 V (Li-ion) when only a cell count is given.
- Usable = E × (depth of discharge − reserve).
- I = P / V; C-rate = I / capacity (Ah).

## Symbols and units

Capacity in mAh or Ah, voltage in V, energy in Wh, power in W, current in A, depth of discharge and reserve in percent.

## Domain

Positive capacity and voltage (or cell count), and a reserve below the depth-of-discharge limit.

## Approximations

Nominal voltage times capacity is the rated energy, the figure airlines and regulations use. The energy a pack actually delivers falls with temperature, age, and discharge rate, which the flight-time tool accounts for separately.

## Worked example

- sourcePublisher: Federal Aviation Administration, Office of Hazardous Materials Safety
- sourceTitle: PackSafe, Batteries Carried by Airline Passengers
- sourceEdition: December 2024
- sourceLocator: Q3, "How do I determine a lithium-ion battery's watt hours (Wh) rating?" (a 12-volt battery rated to 8 Ah is rated at 96 Wh; for mAh, divide by 1,000 and multiply by the volts)
- independent: yes
- inputs: capacity 8 Ah, voltage 12 V
- outputs: 96 Wh
- tolerance: exact
- verifiedBy: golden vector v020, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_drone.py`: a separate Python implementation at 19 packs, from 1,300 mAh to 22,000 mAh and 7.4 V to 51.8 V, with cell counts, depth of discharge and reserve, and current and C-rate (within 1e-9 relative)
- `core/vectors/drone.power.battery-energy.jsonl`: those vectors and the FAA example, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/power_ops.rs` `battery_energy_invariants`: energy is linear in capacity and voltage, Ah and mAh agree, a cell count equals its nominal voltage, and usable energy is the energy between the depth-of-discharge limit and the reserve
