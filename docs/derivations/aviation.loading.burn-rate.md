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

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-aviation-suite, aviation/fuel-and-loading spec, "Fuel planning"
- sourceEdition: 2026
- sourceLocator: Scenario "Burn rate from a flight" (2.5 h, 28 US gal used, 40 US gal usable)
- independent: no
- inputs: fuel used 28 gal, time 2.5 h, usable fuel 40 gal
- outputs: burn rate 11.2 gal/h; endurance about 3.57 h (3.5714285714285716)
- tolerance: 1e-9
- verifiedBy: golden vector v001 and `burn_rate_from_a_flight`
- verifiedOn: 2026-10-09

## Differential tests

- `core/vectors/aviation.loading.burn-rate.jsonl`: the spec scenario and four more flights worked by hand from the two definitions, and a refused zero fuel, run through the core on every build; there is no generator script and no independent implementation, since the tool is a single division

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `burn_rate_from_a_flight`: the spec scenario gives 11.2 gal/h and an endurance of 40 / 11.2 h within 1e-9, and without usable fuel no endurance is returned
