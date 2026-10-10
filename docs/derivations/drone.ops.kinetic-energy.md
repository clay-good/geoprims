<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Drone impact kinetic energy (`drone.ops.kinetic-energy`)

## Method

The tool works out a drone's kinetic energy from its mass and speed, in joules and in foot-pounds, and sets it beside two thresholds that drone rules state in energy: the EASA class C1 requirement and the FAA limit for category 2 operations over people. Both thresholds are read from dated entries in `data/regulations.json`. Both rules judge the energy passed to a person in a test, so the comparison is a screen that shows which way a design leans, not a finding.

## Equations

- Kinetic energy: KE = ½ × mass × speed².
- In foot-pounds: KE ÷ 1.3558179483314004 (1 ft·lbf = 0.3048 m × 4.4482216152605 N).
- EASA C1 screen: a mass under 900 g meets the mass alternative whatever the energy; otherwise KE under 80 J meets the energy criterion; otherwise neither is met.
- FAA category 2 screen: KE above 11 ft·lbf is above the threshold; otherwise at or under it.

## Symbols and units

Mass in any mass unit (kg by default), speed in any speed unit (m/s by default). Energy in joules and in foot-pounds force.

## Domain

Mass above zero; speed zero or more. Other values are INVALID_INPUT.

## Approximations

The energy is exact for the speed entered. The regulations do not ask for this number. C1 asks for the energy transmitted to a human head in an impact at terminal velocity, which need not be the top speed in level flight, and category 2 for an injury no worse than a transfer of 11 foot-pounds from a rigid object would cause, shown by an accepted means of compliance. A drone that tumbles, breaks up, or carries guards transfers less than it carries.

## Worked example

- sourcePublisher: European Commission (EUR-Lex) and Federal Aviation Administration (eCFR)
- sourceTitle: Commission Delegated Regulation (EU) 2019/945, and 14 CFR Part 107
- sourceEdition: Official Journal L 152, 11.6.2019, read on EUR-Lex 2026-10-10; eCFR as in force 2026-09-01, read 2026-10-09
- sourceLocator: 2019/945 Annex Part 2, class C1, point (1): "the energy transmitted to the human head is less than 80 J, or, as an alternative, shall have an MTOM of less than 900 g, including payload"; 14 CFR 107.120(a)(1): "a transfer of 11 foot-pounds of kinetic energy upon impact from a rigid object"
- independent: yes
- inputs: 0.9 kg at 13.3 m/s and at 13.4 m/s; 0.899 kg at 30 m/s; 0.3 kg at 9.9 m/s and at 10.0 m/s
- outputs: 79.6 J, under the C1 energy criterion, and 80.8 J, which cannot meet C1; under 900 g, so the mass alternative is met; 14.70 J (10.84 ft·lbf), at or under the category 2 threshold, and 15.0 J (11.06 ft·lbf), above it
- tolerance: exact for the verdicts; 1e-9 for the energies
- verifiedBy: `kinetic_energy_invariants` (named below) for the verdicts, and golden vectors v006 through v011 for those energies
- verifiedOn: 2026-10-10

The regulations give the thresholds. The energy itself is the definition of kinetic energy.

## Differential tests

- `tools/vectors/gen_drone.py`: the energy in joules and foot-pounds worked again in Python at 20 mass and speed pairs in kilograms, grams, pounds, m/s, km/h, mph, and knots, including pairs just either side of 80 J and of 11 ft·lbf
- `core/vectors/drone.ops.kinetic-energy.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-drone/tests/power_ops.rs` `kinetic_energy_invariants`: energy is linear in mass and quadratic in speed and the same in both units; the C1 screen turns on 900 g and then on 80 J; the category 2 screen turns on 11 ft·lbf
- `core/crates/gp-drone/tests/power_ops.rs` `easa_c1_energy`: 0.9 kg at 19 m/s is 162.45 J and cannot meet C1
