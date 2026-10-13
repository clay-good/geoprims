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

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Airplane Flying Handbook
- sourceEdition: FAA-H-8083-3C (2021)
- sourceLocator: Chapter 7, Figure 7-12, Speed versus pivotal altitude
- independent: yes
- inputs: groundspeed 100 kt
- outputs: approximate pivotal altitude 885 ft AGL
- tolerance: 0.5 ft, because the source prints whole feet
- verifiedBy: golden vector v006 and `core/crates/gp-aviation/tests/slice2.rs` `pivotal_altitude_100_kt`
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_pivotal_altitude.py`: a separate Python evaluation of h = V²/g0 and the knots-squared-over-11.3 rule from 1 to 250 kt, plus refused zero and negative groundspeeds; it restates the formulas, so it checks the core's unit conversions and arithmetic
- `core/vectors/aviation.performance.pivotal-altitude.jsonl`: those cases and the printed FAA example, 20 vectors in all, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `pivotal_altitude_100_kt`: 100 kt gives 885 ft within 0.5 ft, and the rule of thumb is exactly 10,000 / 11.3 ft within 1e-9
- `core/crates/gp-aviation/tests/slice2.rs` `pivotal_altitude_invariants`: doubling groundspeed quadruples the exact and rule-of-thumb heights; equivalent knot and meter-per-second inputs agree; the reported rule error equals rule height minus exact height
