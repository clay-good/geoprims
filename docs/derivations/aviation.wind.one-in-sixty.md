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

- sourcePublisher: UK Ministry of Defence
- sourceTitle: AP3456: The Central Flying School Manual of Flying, Volume 9, Navigation
- sourceEdition: Volume 9, revised March 2011
- sourceLocator: Chapter 19, paragraphs 2-4 and Figure 3
- independent: yes
- inputs: flown 30 NM, off course 2 NM; the tool example also uses 30 NM remaining
- outputs: published track error by the rule 4°; the tool also gives exact track error 3.8140748342903543°, exact turn to the destination 7.628149668580709°, and 8° by the rule
- tolerance: the published rule result is exact to its printed degree; calculated outputs use 1e-9°
- verifiedBy: golden vector v009, run by the core on every build
- verifiedOn: 2026-10-13

AP3456 states that the rule assesses both track error and closing angle, gives the formula, and works the 2 NM over 30 NM example to 4°. The tool applies the same published ratio to the equal 30 NM remaining distance, so the rule gives another 4° to close and an 8° total turn. The exact columns use `atan` on each along-track right triangle rather than the approximation.

## Differential tests

- `tools/vectors/gen_offcourse.py`: a Python evaluation of the same flat-plane angles and rule at 17 valid cases, including the two AP3456 ratios, zero off course, three distance units, and a large 10-in-20 error (within 1e-9), plus three refusals; it restates the formulas, so it checks the code's arithmetic and unit handling
- `core/vectors/aviation.wind.one-in-sixty.jsonl`: those vectors, run through the core on every build

## Invariants

- `core/crates/gp-aviation/tests/aviation.rs` `one_in_sixty_invariants`: scaling all three distances leaves every angle unchanged; changing nautical miles to equivalent kilometers leaves every result unchanged; swapping flown and remaining swaps track error and closing angle but leaves the total turn unchanged; zero offset makes every angle zero
