<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Weight shift (`aviation.loading.weight-shift`)

## Method

Moving weight already aboard changes the total moment but not the total weight. The CG therefore moves by the weight moved times the distance moved, divided by the total weight. Solved one way it gives the new CG for a given weight moved. Solved the other way it gives the weight that must move between two stations to put the CG on a target.

## Equations

- ΔCG = w × (arm_to − arm_from) / W
- New CG = CG + ΔCG
- Weight to move for a target: w = W × (CG_target − CG) / (arm_to − arm_from)

## Symbols and units

W total weight and w weight moved, in pounds (or another mass unit); CG, the arms, and ΔCG in inches (or another length) from the same datum.

## Domain

A weight moved between zero and the total weight, and two different arms. A target the move cannot reach is refused with NO_SOLUTION: a move that carries the CG away from the target, or one that would need more than the whole airplane's weight.

## Approximations

None: this is the exact moment balance, with the total weight unchanged.

## Worked example

- sourcePublisher: Federal Aviation Administration
- sourceTitle: Aircraft Weight and Balance Handbook (FAA-H-8083-1B)
- sourceEdition: 2016
- sourceLocator: Chapter 10, page 10-6: "Determining ΔCG Caused by Shifting Weights" and "Determining Weight Shifted to Cause Specified ΔCG"
- independent: yes
- inputs: 50 lb moved from station 246 to 118 of a 4,709 lb airplane; and the weight to move between the same stations for a CG 2 in forward
- outputs: the CG moves forward 1.36 in; 73.6 lb must move
- tolerance: 0.005 in and 0.05 lb (the printed rounding)
- verifiedBy: golden vectors v008 and v009, run by the core on every build
- verifiedOn: 2026-09-24

The handbook gives no starting CG, and neither answer depends on one; the vectors use 180 in and pin only the change and the weight.

## Differential tests

- `tools/vectors/gen_shift.py`: the proportions evaluated independently in Python at random weights, CGs, and stations, alternating between a given weight and a target CG (within 1e-9)
- `core/vectors/aviation.loading.weight-shift.jsonl`: those vectors, the handbook examples, and the refusals, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `shift_and_ballast_agree_with_weight_and_balance`: the loading rebuilt station by station in `aviation.loading.weight-balance` lands on the shifted CG within 1e-9, and shifting the same weight back restores the original CG
