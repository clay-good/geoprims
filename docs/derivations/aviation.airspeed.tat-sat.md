<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Total and static air temperature (`aviation.airspeed.tat-sat`)

## Method

A temperature probe in moving air reads warmer than the outside air because the air is slowed against it. For an adiabatic stop the temperature rises by the factor 1 + (γ − 1)/2 · M², with γ = 1.4; a real probe recovers only part of that rise, set by its recovery factor r. The tool divides the total (probe) temperature by that factor to get the static temperature, or multiplies the static temperature by it to predict the probe reading. The ram rise is the difference.

## Equations

- f = 1 + 0.2 · r · M²  (0.2 = (γ − 1)/2 with γ = 1.4)
- From a total reading: SAT = TAT / f
- From a static reading: TAT = SAT · f
- Ram rise = TAT − SAT
- All temperatures are absolute (kelvins) in the arithmetic.

## Symbols and units

TAT total air temperature and SAT static air temperature, entered in °C by default and worked in K; M Mach number (0 to 5); r probe recovery factor (0.5 to 1.0, default 1.0); ram rise in K.

## Domain

Temperatures above absolute zero; Mach 0 to 5; recovery factor 0.5 to 1.0. At Mach 0 the two temperatures are equal.

## Approximations

The relation assumes a perfect gas with γ = 1.4 and a single recovery factor for the probe. The default r = 1.0 is full recovery; real probes are about 0.75 to 1.0, so the answer is only as good as the r entered. Probe heating and any correction the air data computer applies are not modeled.

## Worked example

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-aviation-suite, aviation/airspeed spec, "Total and static air temperature"
- sourceEdition: 2026
- sourceLocator: Scenario "Ram rise at Mach 0.8" (TAT −20 °C, M 0.8, r 1.0)
- independent: no
- inputs: temperature −20 °C (total), Mach 0.8, recovery factor 1.0 (default)
- outputs: SAT ≈ −48.73 °C, ram rise ≈ 28.73 K (computed: 253.15 / 1.128 − 273.15 = −48.7262 °C)
- tolerance: 0.005 °C against the spec's rounded values; 1e-6 against the computed value
- verifiedBy: golden vector v001 and `tat_to_sat_ram_rise`
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_aviation.py`: a Python evaluation of the same relation, SAT = TAT / (1 + 0.2 r M²) in kelvins, at five temperatures, Mach numbers, and recovery factors; it restates the formula rather than deriving it another way
- `core/vectors/aviation.airspeed.tat-sat.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `tat_to_sat_ram_rise`: the spec scenario gives SAT −48.73 °C and ram rise 28.73 K (within 0.005), and feeding that SAT back as a static temperature at the same Mach returns TAT −20 °C within 1e-9 (the two directions invert each other)
