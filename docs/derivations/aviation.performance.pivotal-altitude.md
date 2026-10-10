<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Pivotal altitude (`aviation.performance.pivotal-altitude`)

## Method

At the pivotal altitude the line of sight from the pilot to the pylon stays parallel to the airplane's lateral axis through a level turn. For a level turn of radius r at bank φ, tan φ = V²/(g r); the line of sight to the pylon at height h and distance r has the same angle when tan φ = h / r. Equating the two gives h = V²/g, independent of bank. The tool uses the groundspeed for V. The pilot's rule, knots squared over 11.3, is shown beside the exact value with its error.

## Equations

- h = V² / g0, with V the groundspeed in m/s
- Rule of thumb: h_rule = (V in kt)² / 11.3 ft
- Rule error = h_rule − h

## Symbols and units

V groundspeed, entered in knots (1 kt = 1,852/3,600 m/s); g0 = 9.80665 m/s²; h in feet above the ground (1 ft = 0.3048 m).

## Domain

Groundspeed above zero.

## Approximations

Level flight at a steady groundspeed. Around the pylon the groundspeed changes with the wind, so the pivotal altitude changes too. The answer is height above the ground; the pylon's elevation must be added for an altimeter reading. The rule reads about 0.05% low (884.96 ft against 885.40 ft at 100 kt).

## Worked example

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-aviation-suite, aviation/flight-performance spec, "Pivotal altitude and ground-reference geometry"
- sourceEdition: 2026
- sourceLocator: Scenario "Pivotal altitude at 100 kt" (about 885 ft AGL)
- independent: no
- inputs: groundspeed 100 kt
- outputs: pivotal altitude about 885 ft AGL (885.4036845041701 computed); rule 10,000 / 11.3 = 884.96 ft
- tolerance: 0.5 ft against the spec's 885; 1e-6 ft against the computed value
- verifiedBy: golden vector v001 and `pivotal_altitude_100_kt`
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_aviation.py`: a Python evaluation of h = V²/g0 at five groundspeeds (within 1e-12 relative); it restates the formula, so it checks the core's unit conversions and arithmetic
- `core/vectors/aviation.performance.pivotal-altitude.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `pivotal_altitude_100_kt`: 100 kt gives 885 ft within 0.5 ft, and the rule of thumb is exactly 10,000 / 11.3 ft within 1e-9
