<!-- Derivation note (trust/correctness-program layer A). -->
# Cold battery derating (`drone.power.temperature-derating`)

## Method

Lithium packs deliver less energy when cold. Lacking test data for a pack, planners use an allowance. This operation states the one the endurance tools here apply: nothing at room temperature, a fifth at freezing, a straight line between, and never more than half. It is a rule of thumb and says so in its result.

## Equations

- T ≥ 20 °C: derating = 0
- T < 20 °C: derating = (20 − T) × 1 percentage point per °C, capped at 50%
- Share remaining = 100% − derating; energy remaining = E × share

## Symbols and units

T battery temperature in °C, °F, or K; E pack energy in watt-hours (optional).

## Domain

Any temperature. A given energy must be positive.

## Approximations

The whole operation is an approximation, flagged HEURISTIC_DERATING whenever it takes anything off. It is not fitted to any cell chemistry.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-drone-suite specification
- sourceEdition: 2026
- sourceLocator: drone/power, requirement "Endurance" (labeled cold derating); checked by hand arithmetic, not taken from a published table
- independent: no
- inputs: battery at 5 °C, 99 Wh pack
- outputs: 15% derating, 85% remaining, 84.15 Wh
- tolerance: 1e-9
- verifiedBy: golden vector v001 and the `focused_power_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_drone.py` `derating_vectors`: 20 temperatures in °C, °F, and K from −45 °C to 35 °C, with and without a pack energy, by a separate Python evaluation of the rule.
- `core/vectors/drone.power.temperature-derating.jsonl`: those vectors and one refusal case.
- The integration suite checks the fixed points (20 °C, 0 °C, the 50% cap) and the warning.

## Invariants

- The derating never rises with temperature.
- It is 0 at 20 °C, 20% at 0 °C, and 50% at −30 °C and below.
- 32 °F gives the same result as 0 °C.
