<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Specific range (`aviation.performance.specific-range`)

## Method

The definition of specific range in the Pilot's Handbook of Aeronautical Knowledge: distance flown per unit of fuel. Through the air it is the true airspeed over the fuel flow; over the ground it is the groundspeed over the fuel flow. The fuel each 100 NM takes is the reciprocal times 100. The ground figures appear only when a groundspeed is given.

## Equations

- Air range = TAS / fuel flow (NM per US gallon)
- Fuel per 100 NM through the air = 100 × fuel flow / TAS
- Ground range = GS / fuel flow
- Fuel per 100 NM over the ground = 100 × fuel flow / GS

## Symbols and units

TAS and GS in knots; fuel flow in US gallons per hour (liters per hour and other flow units are converted, at 3.785411784 L per US gallon); ranges in NM per US gallon; fuel in US gallons by default.

## Domain

A true airspeed and a fuel flow above zero; a groundspeed, when given, above zero. Anything else is INVALID_INPUT.

## Approximations

None in the division. The figures are only as good as the fuel flow entered, from the POH or a gauge. The tool does not search for the best power setting, and it leaves out climb, descent, and reserve, which `aviation.loading.fuel-plan` covers.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge
- sourceEdition: FAA-H-8083-25C (2023)
- sourceLocator: Chapter 16, Navigation, fuel-consumption example and the following specific-range note
- independent: yes
- inputs: 400 NM, 100 kt, 5 gal/h
- outputs: 4 hours and 20 gal are printed by the source; those independently published values give 20 NM/gal and 5 gal per 100 NM
- tolerance: 1e-12 for the exact ratios
- verifiedBy: golden vector v009 and `core/crates/gp-aviation/tests/slice2.rs` `specific_range_matches_phak_example`
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_range.py`: the same divisions written in Python across 13 speed, flow, groundspeed, and unit combinations, plus six zero or negative invalid cases. It checks the code against the definition, not against a separate method
- `core/vectors/aviation.performance.specific-range.jsonl`: those cases and the independently published FAA example, 20 vectors in all, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `specific_range_matches_phak_example`: the FAA's 400 NM, 100 kt, 5 gal/h, 20-gallon trip gives 20 NM/gal and 5 gal per 100 NM
- `core/crates/gp-aviation/tests/slice2.rs` `specific_range_invariants`: doubling speed at fixed flow doubles range and halves fuel per 100 NM; doubling flow does the reverse; changing groundspeed leaves the air results unchanged; equivalent metric inputs reproduce all four outputs
