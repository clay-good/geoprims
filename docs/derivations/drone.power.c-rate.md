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

- sourcePublisher: geoprims
- sourceTitle: add-drone-suite specification
- sourceEdition: 2026
- sourceLocator: drone/power, requirement "Battery energy" (current and C-rate for a draw); checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: 10,000 mAh, 6 cells LiPo, 480 W
- outputs: 22.2 V nominal, 21.62 A, 2.16 C
- tolerance: 1e-12
- verifiedBy: golden vector v001 and the `focused_power_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `c_rate_vectors`: 20 packs from 850 mAh to 30 Ah, by voltage or by cell count in both chemistries, by a separate Python evaluation.
- `core/vectors/drone.power.c-rate.jsonl`: those vectors and three refusal cases.
- The integration suite requires the same current, C-rate, and warnings as `drone.power.battery-energy`.

## Invariants

- The C-rate is proportional to the power and inversely proportional to the capacity and the voltage.
- Current × voltage = power.
- Doubling the capacity halves the C-rate.
