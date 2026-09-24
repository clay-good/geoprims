<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Ballast (`aviation.loading.ballast`)

## Method

Ballast adds weight at a chosen arm, so the moments must balance about the target CG. The airplane's weight times its distance from the target equals the ballast times the ballast's distance from the target. That gives the ballast weight directly. The new total weight is reported beside it.

## Equations

- W × (CG_target − CG) = B × (arm − CG_target)
- B = W × (CG_target − CG) / (arm − CG_target)
- New weight = W + B

## Symbols and units

W total weight and B ballast, in pounds (or another mass unit); CG, the target, and the ballast arm in inches (or another length) from the same datum.

## Domain

A ballast arm different from the target CG. An arm on the wrong side of the target, which would need negative ballast, is refused with NO_SOLUTION.

## Approximations

None: this is the exact moment balance.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Aircraft Weight and Balance Handbook (FAA-H-8083-1B)
- sourceEdition: 2016
- sourceLocator: Chapter 10, page 10-7: "Determining Amount of Ballast Needed to Move CG to a Desired Location"
- independent: yes
- inputs: a 1,876 lb airplane with its CG at +32.2, target +33 (its forward limit), ballast at station 228
- outputs: 7.7 lb of ballast
- tolerance: 0.05 lb (the printed rounding)
- verifiedBy: golden vector v006, run by the core on every build
- verifiedOn: 2026-09-24

## Differential tests

- `tools/vectors/gen_shift.py`: the balance evaluated independently in Python at random weights, CGs, targets, and arms on both sides (within 1e-9)
- `core/vectors/aviation.loading.ballast.jsonl`: those vectors, the handbook example, and the refusal, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `shift_and_ballast_agree_with_weight_and_balance`: the airplane and the computed ballast entered as two stations in `aviation.loading.weight-balance` land exactly on the target CG, within 1e-9
