<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Off-course correction (1-in-60) (`aviation.wind.one-in-sixty`)

## Method

On a flat plane, a straight track that ends a distance off the course line after a distance flown makes the track error angle with the course. Turning back by the track error parallels the course. The closing angle is the angle from the parallel to a line straight to the destination, set by the distance off and the distance remaining. Turning by the sum of the two reaches the destination. The 1-in-60 rule (1 NM off in 60 NM is about 1°) is shown beside each exact angle.

## Equations

- Track error = atan(off / flown)
- Closing angle = atan(off / remaining)
- Turn to the destination = track error + closing angle
- Rule: track error ≈ 60 × off / flown; closing angle ≈ 60 × off / remaining; turn ≈ their sum

## Symbols and units

Distances flown, off course, and remaining in nautical miles (any distance unit is accepted and converted); angles in degrees.

## Domain

Distances flown and remaining above zero; distance off course zero or more (given as positive; the turn is back toward the course).

## Approximations

Flat-plane geometry, fine for legs of a few hundred miles, and a straight track from the start. The rule uses tan θ ≈ θ in radians and 60 in place of 57.3, so it reads about 5% high under 10° and further off above 20° (60° against 53.13° in the vectors' 10-off-in-20 case).

## Worked example

- sourcePublisher: geoprims (spec scenario, computed)
- sourceTitle: add-aviation-suite, aviation/wind-and-navigation spec, "Off-course correction (1-in-60)"
- sourceEdition: 2026
- sourceLocator: Scenario "4 NM off after 60 NM" (track error about 3.81°, rule 4°; turn to the destination about 7.63°, rule 8°)
- independent: no
- inputs: flown 60 NM, off course 4 NM, remaining 60 NM
- outputs: track error 3.81° (3.8140748342903543), closing angle 3.81°, turn to the destination 7.63°; rule 4°, 4°, 8°
- tolerance: 1e-9°
- verifiedBy: golden vector v001
- verifiedOn: 2026-10-09

## Differential tests

- `tools/vectors/gen_offcourse.py`: a Python evaluation of the same flat-plane angles and rule at six cases, including zero off course and a large 10-in-20 error (within 1e-9), plus two refusals; it restates the formulas, so it checks the code's arithmetic and unit handling
- `core/vectors/aviation.wind.one-in-sixty.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `golden_vectors`: no test checks an invariant of this tool; this is the closest test that exercises it, and it runs every vector in the tool's file through the core and fails on any value outside its tolerance
