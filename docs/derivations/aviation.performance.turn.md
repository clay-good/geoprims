<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Turn performance and load factor (`aviation.performance.turn`)

## Method

A coordinated level turn at constant true airspeed. Lift tilted by the bank angle φ carries the weight vertically and supplies the centripetal force horizontally, so tan φ = V·ω ÷ g0. Any two of true airspeed, bank, and turn rate give the third; with true airspeed alone the turn rate defaults to standard rate, 3° per second. From V and φ come the turn rate, the radius, the load factor, and the time for a heading change. The stall speed in the turn scales a user-entered 1 g stall speed by √n, and a limit load factor gives the steepest bank within it. For a standard-rate turn the KTAS ÷ 10 + 7 bank rule is shown beside the exact bank.

## Equations

- Bank from speed and rate: φ = atan(V · ω ÷ g0); speed from bank and rate: V = g0 · tan φ ÷ ω.
- Turn rate: ω = g0 · tan φ ÷ V.
- Radius: r = V² ÷ (g0 · tan φ); in feet and knots r = V² ÷ (11.294 · tan φ), from g0 (the textbook 11.26 is a rounded variant).
- Load factor: n = 1 ÷ cos φ.
- Time for a heading change Δψ (default 360°): t = |Δψ| ÷ ω.
- Stall speed in the turn: Vs · √n.
- Maximum bank for a load limit: acos(1 ÷ n_limit); LOAD_LIMIT_EXCEEDED when n > n_limit.
- Rule of thumb (standard rate only): φ_rule = KTAS ÷ 10 + 7, with its error φ_rule − φ.

## Symbols and units

g0 = 9.80665 m/s². V is true airspeed (knots by default, computed in m/s); φ bank in degrees; ω turn rate in degrees per second; r in feet; t in seconds; n dimensionless; Vs the 1 g stall speed (CAS, from the POH).

## Domain

0° < φ < 90° (otherwise OUT_OF_DOMAIN); V > 0 and ω > 0 when given; exactly two of V, φ, ω (or V alone for standard rate); limit load factor from 1 to 12.

## Approximations

The turn is level, coordinated, and at constant true airspeed. Climbing, descending, or slipping turns differ, and wind changes the ground path but not these numbers. The stall speed in the turn is a scaling of the entered 1 g value, not a POH figure. The bank rule is a rough guide: 17° against 15.36° at 100 KTAS.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge
- sourceEdition: FAA-H-8083-25C (2023)
- sourceLocator: Chapter 5, Figure 5-56, rate of turn for a given true airspeed and bank angle
- independent: yes
- inputs: TAS 120 kt, bank 30°
- outputs: turn rate 5.25°/s
- tolerance: 0.005°/s, the half-unit of the source's second decimal place
- verifiedBy: golden vector v007 and `core/crates/gp-aviation/tests/slice2.rs` `turn_matches_phak_example`
- verifiedOn: 2026-10-13

The handbook also prints a 2,215 ft radius from its rounded 11.26 constant. The tool uses standard gravity and exact unit definitions, producing 2,208 ft; the model and accuracy text disclose that difference, so the rounded radius is not used as an expected value.

## Differential tests

- `tools/vectors/gen_turn.py`: the turn relations written separately in Python across standard-rate, given-bank, given-rate, inverse-speed, heading-change, stall-speed, and load-limit cases, plus refused invalid inputs. It checks the code against the stated equations, not against a separate method
- `core/vectors/aviation.performance.turn.jsonl`: those cases and the independently printed FAA turn rate, 20 vectors in all, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/slice2.rs` `turn_matches_phak_example`: 120 KTAS at 30° bank gives 5.25°/s within the source's printed precision and a 360° turn in 68.6 seconds within 0.1 second
- `core/crates/gp-aviation/tests/slice2.rs` `turn_solves_for_tas`: the bank from a standard-rate turn at 100 kt, given back with a 3°/s rate, solves to 100 kt (within 1e-6)
- `core/crates/gp-aviation/tests/slice2.rs` `sixty_degree_bank`: a 60° bank gives a load factor of 2 (within 1e-12), a 50 kt stall speed rises to 70.7 kt, the radius matches the 11.294 feet-and-knots form, and no bank rule is shown for a non-standard turn
- `core/crates/gp-aviation/tests/slice2.rs` `load_limit_exceeded`: at a 3.8 g limit the maximum bank is acos(1 ÷ 3.8), a 77° bank warns, and a 45° bank does not
