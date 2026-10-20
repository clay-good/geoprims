<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Distance off by two bearings on the bow (`navigation.piloting.two-bearing-fix`)

## Method

Bowditch Table 18's plane oblique and right triangles: from two bearings relative to the bow and the run between them, the distance off at the second bearing and the predicted distance when abeam. Doubling the angle on the bow (B = 2A) and the bow-and-beam bearing (45° and 90°) are presets.

## Equations

- d₂ = run × sin A / sin(B − A).
- Abeam = d₂ sin B.
- Doubling: d₂ = run. Bow and beam: abeam = run.

## Symbols and units

Relative bearings in degrees off the bow; run in nautical miles (or speed × time).

## Domain

0 < A < B < 180°, on the same side.

## Approximations

No current and a steady course and speed, as the result states; Table 18 prints its factors to 0.01.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), Pub. No. 9
- sourceEdition: 2024 edition
- sourceLocator: §1121 and Volume II Table 18
- independent: no (the scenario is the change's own; Bowditch's §1121 example is vector v001 of the running fix)
- inputs: Doubling the angle: 30° and 60° on the bow, 4.0 NM apart (the change's scenario)
- outputs: 4.0 NM at the second bearing and 3.46 NM abeam
- tolerance: 1e-9 NM and 0.01 NM
- verifiedBy: golden vector v001 and `core/crates/gp-navigation/tests/piloting.rs`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_piloting.py two-bearing-fix`: Python triangle at 19 random bearing pairs (1e-9 NM) and the reversed-order refusal.
- `core/vectors/navigation.piloting.two-bearing-fix.jsonl`: those vectors, run through the core on every build.

## Invariants

- `bearings_and_fixes`: doubling gives the run; bow and beam gives the run abeam.
