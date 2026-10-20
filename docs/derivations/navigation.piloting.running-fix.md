<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Running fix from one object (`navigation.piloting.running-fix`)

## Method

Two true bearings of one charted object and the course and run between them. The relative bearings A and B fix the triangle of Bowditch Table 18, which is the solution of advancing the first line of position by the run and crossing it with the second (§1112, §1121). With the object's position, the fix is placed from the object along the reciprocal of the second bearing by the WGS 84 geodesic, and both lines of position are drawn.

## Equations

- A = bearing₁ − C, B = bearing₂ − C (same side).
- Distance at the second bearing d₂ = run × sin A / sin(B − A); abeam d₂ sin B.
- Fix = object + geodesic(bearing₂ + 180°, d₂).

## Symbols and units

Bearings and course in degrees true; run in nautical miles (or speed × time); positions in degrees.

## Domain

Both bearings on the same side of the course, the second broader on the bow than the first and under 180°.

## Approximations

The course and speed made good between the bearings must be known; an unknown current moves the fix. On these scales the geodesic and the chart's straight line coincide.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), Pub. No. 9
- sourceEdition: 2024 edition
- sourceLocator: §1121, the Table 18 example
- independent: yes
- inputs: Course 050° at 15 kn, a light at 024° at 1130 and 359° at 1140
- outputs: 2.6 NM at 1140 and 2.0 NM abeam (Table 18 factors 1.04 and 0.81 on a 2.5 NM run)
- tolerance: 0.05 NM
- verifiedBy: golden vector v001 and `core/crates/gp-navigation/tests/piloting.rs`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_piloting.py running-fix`: Python Table 18 triangle with pyproj.Geod placing the fix at 19 random cases (1e-7°) and the reversed-bearings refusal.
- `core/vectors/navigation.piloting.running-fix.jsonl`: those vectors, run through the core on every build.

## Invariants

- `bearings_and_fixes`: the §1121 example; the fix lies at that distance from the object, on the second bearing within the meridians' convergence.
