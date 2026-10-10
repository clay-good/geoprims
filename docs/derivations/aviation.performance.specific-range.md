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

- sourcePublisher: geoprims (worked from the definition, computed)
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C), definition applied by hand
- sourceEdition: FAA-H-8083-25C
- sourceLocator: Aircraft Performance chapter, range performance and specific range; the handbook gives the definition, and these numbers are the project's own division, not a handbook example
- independent: no
- inputs: TAS 120 kt, fuel flow 8.5 gal/h, groundspeed 105 kt
- outputs: 14.12 NM/gal through the air, 7.08 gal per 100 NM; 12.35 NM/gal over the ground, 8.10 gal per 100 NM
- tolerance: 1e-9
- verifiedBy: golden vector v001
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_range.py`: the same divisions written in Python, at five cases (one with no groundspeed, one with a liters-per-hour flow) and three refused zero inputs (within 1e-9)
- `core/vectors/aviation.performance.specific-range.jsonl`: those eight vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `golden_vectors`: no invariant test is written for this tool yet. This is the closest test: it lints and runs every aviation tool's golden vectors through the registry, including the eight above, and fails if any tool has fewer than five
