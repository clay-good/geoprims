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

- sourcePublisher: flightcondition (open-source Python library by Matthew C. Jones)
- sourceTitle: flightcondition 26.4.20, FlightCondition(h, M): static temperature T and total temperature T0
- sourceEdition: flightcondition 26.4.20, run 2026-10-10
- sourceLocator: tools/vectors/gen_tat_sat_fc.py, first case: FlightCondition(h=35000 ft, M=0.8)
- independent: yes
- inputs: total temperature 246.94647 K (−26.20 °C), Mach 0.8, recovery factor 1.0 (default)
- outputs: the library's static temperature at that height, 218.92418 K (−54.23 °C); ram rise 28.02 K
- tolerance: 1e-9 K
- verifiedBy: golden vector v006, run by the core on every build
- verifiedOn: 2026-10-10

The library gives 15 more cases, v007 through v021, from sea level to 51,000 ft and Mach 0.2 to 0.92, in both directions. In seven of them the probe temperature is the library's wall recovery temperature in turbulent or laminar flow, and the recovery factor entered is the library's own, (Tr − T) ÷ (T0 − T), 0.888 or 0.837. Those seven show that the tool scales the ram rise by the recovery factor the same way the library does; they do not test any particular probe's factor.

## Differential tests

- `tools/vectors/gen_tat_sat_fc.py`: static temperature from total or recovery temperature, and the reverse, against flightcondition at 16 heights and Mach numbers (within 1e-9 K)
- `tools/vectors/gen_aviation.py`: a Python evaluation of the same relation, SAT = TAT / (1 + 0.2 r M²) in kelvins, at five temperatures, Mach numbers, and recovery factors; it restates the formula rather than deriving it another way
- `core/vectors/aviation.airspeed.tat-sat.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice3.rs` `tat_sat_invariants`: over five temperatures, four recovery factors, and six Mach numbers up to 2, the reading is handed back, the ram rise is the gap between the two temperatures and is zero in still air, it grows with Mach and with the recovery factor, and the static temperature given back returns the reading
- `core/crates/gp-aviation/tests/slice2.rs` `tat_to_sat_ram_rise`: the spec scenario gives SAT −48.73 °C and ram rise 28.73 K (within 0.005), and feeding that SAT back as a static temperature at the same Mach returns TAT −20 °C within 1e-9
