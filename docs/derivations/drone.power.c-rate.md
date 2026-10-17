<!-- Derivation note (trust/correctness-program layer A). -->
# Battery C-rate for a power draw (`drone.power.c-rate`)

## Method

A battery's C-rate is its current measured against its capacity: 1 C would empty it in an hour. The current for a power draw follows from the pack voltage. The operation shares one calculation path with `drone.power.battery-energy`, which reports the same two figures when given a draw.

## Equations

- Current I = P / V
- C-rate = I ÷ capacity in amp-hours
- From a cell count: V = cells × 3.7 V (LiPo) or 3.6 V (Li-ion)

## Symbols and units

P power draw in watts; V nominal pack voltage; capacity in mAh or Ah. The C-rate is a plain number.

## Domain

Positive capacity, voltage, and power. A voltage or a cell count, not both. C_RATE_EXCEEDED warns when the rate is over a given continuous rating; NOMINAL_VALUE_USED notes a voltage taken from the cell count.

## Approximations

Nominal voltage is used. Under load, in the cold, or late in a flight the voltage is lower and the real current higher.

## Worked example

- sourcePublisher: Cadex Electronics (Battery University)
- sourceTitle: BU-402: What Is C-rate?
- sourceEdition: retrieved 2026-10-17
- sourceLocator: a 1 Ah battery discharging at 2C delivers 2 A (current = C-rate × capacity)
- independent: yes
- inputs: capacity 1 Ah, voltage 3.7 V, power 7.4 W
- outputs: current 2 A, C-rate 2 C
- tolerance: exact
- verifiedBy: the `focused_power_operations_match_their_general_tools` integration test, which requires the current and C-rate to equal the stable `drone.power.battery-energy` tool's for the same draw
- verifiedOn: 2026-10-17

## Differential tests

- `tools/vectors/gen_drone.py` `c_rate_vectors`: 20 packs from 850 mAh to 30 Ah, by voltage or by cell count in both chemistries, by a separate Python evaluation.
- `core/vectors/drone.power.c-rate.jsonl`: those vectors and three refusal cases.
- The integration suite requires the same current, C-rate, and warnings as `drone.power.battery-energy`.

## Invariants

- `core/crates/gp-drone/tests/drone.rs` `focused_power_operations_match_their_general_tools`: the current, C-rate, and nominal voltage equal the stable `drone.power.battery-energy` tool's for the same draw, and both flag a C-rate over the pack's rating.
- Current × voltage = power.
- Doubling the capacity halves the C-rate.
