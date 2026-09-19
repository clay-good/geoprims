<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Runway crosswind and headwind (`aviation.wind.runway-components`)

## Method

Resolve the wind vector onto the runway axis. The angle between the wind direction and the runway heading gives the headwind (along the runway) and crosswind (across it) components. A gust is resolved the same way. A variable wind takes the worst case: the full speed as crosswind and as tailwind.

## Equations

- θ = wind direction − runway heading.
- Headwind = W cos θ (negative is a tailwind).
- Crosswind = W |sin θ|, from the right when sin θ > 0 and from the left when sin θ < 0.

## Symbols and units

W wind speed (kt by default), θ wind angle (degrees). A runway number is taken as its heading ×10°. METAR winds are magnetic-vs-true ambiguous, so a METAR wind on a magnetic runway needs the variation.

## Domain

Any wind direction (0–360°) and speed of 0 kt or more. Calm (00000KT) returns zero components.

## Approximations

None in the vector math. A runway number rounds the heading to 10°, so components can be off by up to W sin 5°. The RUNWAY_HEADING_APPROXIMATE warning says so and asks for the published heading.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
- sourceEdition: 2023
- sourceLocator: Chapter 11, sample problem 10 and the crosswind component chart (runway 17, wind 140° at 25 kt; headwind 22 kt and crosswind 13 kt read from the chart)
- independent: yes
- inputs: runway 17, wind 140° at 25 kt
- outputs: headwind 22 kt, crosswind 13 kt (the core gives 21.7 kt and 12.5 kt)
- tolerance: 1 kt (the chart is read to whole knots)
- verifiedBy: golden vector v021, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_aviation.py`: an independent Python implementation of the component formulas at 20 runway and wind pairs covering all quadrants, pure headwind, and pure tailwind (within 1e-12 relative)
- `core/vectors/aviation.wind.runway-components.jsonl`: those vectors plus the FAA chart example, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `wind_invariants`: headwind² + crosswind² equals the wind speed², and a wind mirrored about the runway gives the same components from the opposite side
