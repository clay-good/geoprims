<!-- Derivation note (trust/correctness-program layer A). -->
# Crandall-rule traverse adjustment (`survey.cogo.crandall`)

## Method

The operation fixes the shared traverse engine to the Crandall rule. Bearings are held as measured and all correction is placed into the course distances. A 2 × 2 weighted normal system solves 2 multipliers from the latitude and departure misclosures. Each course receives a scalar distance correction along its measured direction, so its adjusted bearing is unchanged.

## Equations

For latitude Li, departure Di, and distance Si, form:

- A = Σ(Li²/Si), B = Σ(LiDi/Si), and C = Σ(Di²/Si)
- solve A k1 + B k2 = −ΣL and B k1 + C k2 = −ΣD
- wi = (Li k1 + Di k2) / Si
- corrected components are L'i = Li(1 + wi) and D'i = Di(1 + wi)
- corrected distance is S'i = Si(1 + wi)

## Symbols and units

Li and Di are the unadjusted plane components of course i. Si is its horizontal distance. The multipliers k1 and k2 satisfy the 2 closure constraints. The dimensionless wi scales both components of a course equally, which preserves its bearing. Coordinates and distances use the common input linear unit.

## Domain

The input is a closed plane traverse with 2 to 2,000 positive horizontal distances and courses in at least 2 directions. A traverse confined to one line makes the normal system singular and is refused. All directions must share one grid or assumed north reference, and all linear values must use compatible units.

## Approximations

This is the closed-form Crandall rule, not a general network least-squares adjustment. It assumes bearings should be held and does not estimate observation residuals, covariance, or blunders. The calculation treats courses as plane lines and requires slope, atmospheric, and grid-to-ground reductions to be completed first.

## Worked example

- sourcePublisher: GeoPrims project
- sourceTitle: add-survey-suite Crandall-rule requirement
- sourceEdition: 2026-10-13
- sourceLocator: traverse-adjustments requirement
- independent: no
- inputs: six courses at azimuths 15°, 75°, 130°, 200°, 260°, and 320°
- outputs: every adjusted course keeps its measured bearing and the final coordinate closes exactly on the start
- tolerance: 1 part in 1 billion with a 1e-9 ft absolute allowance
- verifiedBy: golden vector v001 and crandall_holds_every_bearing
- verifiedOn: 2026-10-13

## Differential tests

- `core/vectors/survey.cogo.crandall.jsonl` compares 20 successful traverses with an independent Python solution of the 2 × 2 normal equations and includes a singular one-line case.

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `named_traverse_adjustments_match_the_general_tool` requires the named operation to match the general traverse tool with `adjustment` fixed to `crandall`.
- `core/crates/gp-survey/tests/survey.rs` `crandall_holds_every_bearing` proves every adjusted bearing is unchanged and rotating the grid leaves the corrected distances unchanged.
