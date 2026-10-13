<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Fuel burn rate (`aviation.loading.burn-rate`)

## Method

The average burn rate of a flight is the fuel it used divided by the time it took. When the usable fuel on board is given, the endurance at that rate is the fuel divided by the rate.

## Equations

- Burn rate = fuel used / time
- Endurance = usable fuel / burn rate (only when usable fuel is given)

## Symbols and units

Fuel used and usable fuel are volumes, US gallons by default; time in hours by default; burn rate in gal/h; endurance in hours. Arithmetic is in base units and converted for display.

## Domain

Fuel used and time above zero; usable fuel zero or more.

## Approximations

None in the arithmetic. The rate is an average over the whole flight, so taxi, run-up, and climb raise it on short flights, and Hobbs and tach time give different rates. No reserve is added.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge
- sourceEdition: FAA-H-8083-25C (2023)
- sourceLocator: Chapter 16, Fuel Consumption: a 400 NM flight at 100 kt takes 4 h, and at 5 gal/h it uses 20 gal
- independent: yes
- inputs: fuel used 20 gal, time 4 h
- outputs: burn rate 5 gal/h
- tolerance: exact to the printed values
- verifiedBy: golden vector v007, run by the core on every build
- verifiedOn: 2026-10-13

The handbook multiplies the printed rate and time to get the fuel used. The tool runs the same example in reverse, dividing its 20 gal by 4 h to recover the handbook's 5 gal/h. Three more vectors use exact rows from the FAA Middle Aviation Curriculum Guide's fuel-consumption answer table.

## Differential tests

- `tools/vectors/gen_burn_rate.py`: a separate Python evaluation at 13 additional rates in gallons and liters, hours and minutes, with and without usable fuel, plus three refused inputs
- `core/vectors/aviation.loading.burn-rate.jsonl`: those cases and four FAA printed examples, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `burn_rate_invariants`: scaling fuel used and usable fuel together scales the rate but leaves endurance alone; doubling time halves the rate and doubles endurance; equivalent gallons/hours and liters/minutes agree
