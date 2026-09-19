<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Circular curve (`survey.curves.circular-curve`)

## Method

A simple circular curve is fixed by any two independent elements. With R and Δ, every other element follows directly. With R and one other element, the half-angle comes from inverting that element's formula. With two elements other than R, the half-angle solves element(a)/element(b) = a/b. The solver scans the half-angle from 0° to 90° for every sign change and bisects each one, because tangent over middle ordinate is not monotonic: a given T and M fit two curves, and the tool returns the flatter one and names the other with AMBIGUOUS_INPUT. The degree of curve may be given by the arc definition (100 ft of arc) or the chord definition (100 ft chord). A PI station gives the PC and PT stations.

## Equations

- T = R tan(Δ/2), L = R Δ, C = 2R sin(Δ/2).
- E = R(sec(Δ/2) − 1) = 2R sin²(Δ/4)/cos(Δ/2), M = R(1 − cos(Δ/2)) = 2R sin²(Δ/4). These forms avoid cancellation for flat curves.
- Arc definition: R = 5,729.578 ft / D. Chord definition: R = 50 ft / sin(D/2).
- PC = PI − T, PT = PC + L (stationed along the arc).

## Symbols and units

R radius, Δ deflection (central) angle, T tangent, L arc length, C long chord, E external, M middle ordinate, D degree of curve. Lengths are in the unit entered. US survey and international feet are not mixed. Stations are written 12+34.56 (or 1+234.567 in meters).

## Domain

Exactly two independent elements. R, the arc degree, and the chord degree all fix the radius, so only one of them may be given. Δ is between 0° and 360° when given directly, and 0° to 180° when solved from two lengths. A pair that no curve satisfies is refused.

## Approximations

None: exact circular geometry. With the chord definition, some practice stations the curve along 100 ft chords (L = 100 Δ / D). This tool stations along the true arc, so the PT can differ from chord stationing by the arc-chord difference (0.86 ft on the FM 5-233 45° example).

## Worked example

- sourcePublisher: Headquarters, Department of the Army
- sourceTitle: FM 5-233, Construction Surveying
- sourceEdition: 1985
- sourceLocator: Chapter 3, simple curves: PI 18+00, I = 45°, D = 15° (chord definition), first stake 16+50 after an 8.67 ft subchord, so PC 16+41.33; I = 75°, D = 15°: T 293.11 and E 99.50 (arc, from its 5,730 ft tables) and T 293.94 and E 99.79 (chord); I = 42°15′, D = 5°37′: L = 752.23 ft
- independent: yes
- inputs: Δ 45°, chord degree 15°, PI 18+00
- outputs: PC 16+41.33 (and the other three examples above)
- tolerance: exact station; 0.02 ft for table-based T and E; 0.005 ft for L
- verifiedBy: golden vectors v017 to v020, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_survey.py`: a separate Python implementation of the curve formulas (Ghilani and Wolf) at 12 curves solved from 11 different element pairs, from Δ = 1° to 150°, plus the chord definition (within 1e-9 relative)
- `core/vectors/survey.curves.circular-curve.jsonl`: those vectors, the published examples, and refusals, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `circular_curve_invariants`: all ten pairs of T, L, C, E, and M give back the same R and Δ (only T with M warns of a second curve), and the arc and chord degrees give R = 5,729.578/D and R = 50/sin(D/2); `circular_curve_extreme_angles` solves pairs at Δ = 0.1° and 179.9°
