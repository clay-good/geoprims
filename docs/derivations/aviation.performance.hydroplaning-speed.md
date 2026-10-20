<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Tire hydroplaning speed (`aviation.performance.hydroplaning-speed`)

## Method

Above a certain groundspeed, a tire on a flooded runway can ride up on the water and lose braking and steering. NASA TN D-2056 (Horne and Dreher, 1963) gives that speed from the tire inflation pressure alone, as 9 times the square root of the pressure in psi, in knots (equation 4). The tool uses that by default. It also shows the Airplane Flying Handbook's 8.6 × √p, and returns that one as the answer when the user picks the `afh` formula. Both values are always reported, so their difference shows.

## Equations

- NASA (default): V_P = 9 √p (TN D-2056 eq. 4).
- Handbook option: V_P = 8.6 √p.
- The answer is the NASA value unless the formula choice is `afh`.

## Symbols and units

p is the tire inflation pressure in psi by default (any pressure unit accepted; it is converted to psi before the square root). V_P is a groundspeed in knots, shown to one decimal.

## Domain

p from 24 to 150 psi inclusive, else OUT_OF_DOMAIN at `/pressure`, with the message that NASA TN D-2056 correlated the formula for 24 to 150 psi. The formula choice is `nasa` or `afh`; leaving it out gives `nasa`.

## Approximations

The report found reasonable agreement with tests on tires from 925 to 22,000 lb load and 24 to 150 psi. The formula holds for smooth and closed-pattern treads, and for rib treads only where the water is deeper than the tread grooves. It ignores tread wear, water viscosity, and runway texture. It is an estimate, not a guarantee. A tire that is already hydroplaning can keep doing so below this speed. The report gives no spin-down figure for a lower speed, so the tool shows none.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Pilot's Handbook of Aeronautical Knowledge
- sourceEdition: FAA-H-8083-25C
- sourceLocator: Chapter 11, Water on the Runway and Dynamic Hydroplaning, the text with Figure 11-18: a 36 psi main gear tire begins hydroplaning at 54 knots
- independent: yes
- inputs: tire pressure 36 psi
- outputs: speed 54.0 kt (NASA, 9 × √36), handbook variant 51.6 kt
- tolerance: 1e-12 relative, 1e-9 absolute
- verifiedBy: golden vector v001 and `core/crates/gp-aviation/tests/physiology.rs` `hydroplaning_invariants`
- verifiedOn: 2026-10-20

NASA TN D-2056 prints the equation; the handbook prints the 36 psi example, read at faa.gov on 2026-10-20.

## Differential tests

- No generator script in `tools/vectors/` names this tool. The vectors' source field says they were worked in Python from NASA TN D-2056 eq. 4, V_P = 9 √p kt. The arithmetic is one square root, so this checks unit handling, the formula option, and the range, not a separate method.
- `core/vectors/aviation.performance.hydroplaning-speed.jsonl`: 21 vectors. The 36 psi example (v001); both ends of the range, 24 and 150 psi; pressures between them; the `afh` option (v002) and an explicit `nasa` choice (v018); a pressure in bar (v017); and three OUT_OF_DOMAIN refusals at 23.9, 200, and 0 psi.

## Invariants

- `core/crates/gp-aviation/tests/physiology.rs` `hydroplaning_invariants`: 36 psi gives exactly 54 kt; four times the pressure gives twice the speed (100 psi against 25 psi); and 200 psi is refused with OUT_OF_DOMAIN.
