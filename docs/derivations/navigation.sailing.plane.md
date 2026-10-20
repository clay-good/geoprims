<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Plane sailing (`navigation.sailing.plane`)

## Method

Bowditch's plane sailing: the run is a flat right triangle with the distance D as hypotenuse, the difference of latitude l (north positive) along the meridian, and the departure p (east positive) along the parallel. Either pair can be given and the other is solved.

## Equations

- l = D cos C, p = D sin C.
- tan C = p / l (taken by atan2 so the quadrant follows the signs), D = √(l² + p²).

## Symbols and units

C course (degrees true); D, l, p in nautical miles (l equals minutes of latitude).

## Domain

Any course; a distance of zero or more; l and p of any sign. A course and distance, or l and p, must be given.

## Approximations

Flat: Bowditch limits plane sailing to runs where the Earth's curvature does not matter, a few hundred miles at most. It gives no longitude; parallel or mid-latitude sailing turns p into DLo.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), Pub. No. 9, Volume I
- sourceEdition: 2024 edition
- sourceLocator: Bowditch 2024 §1302: 188.0 NM on course 005°
- independent: yes
- inputs: course 005°, distance 188.0 NM
- outputs: l = 187.3' N, p = 16.4 NM E
- tolerance: 0.05 NM (the printed tenth)
- verifiedBy: golden vector v001 and `bowditch_chapter_13_examples`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_sailings.py plane`: Python reimplementation of both directions at 19 random cases (exact to 1e-9 NM) and the missing-input refusal.
- `core/vectors/navigation.sailing.plane.jsonl`: those vectors with Bowditch's example first, run through the core on every build.

## Invariants

- `core/crates/gp-navigation/tests/sailing.rs` `bowditch_chapter_13_examples`: the §1302 example, and l and p turned back into 005° and 188.0 NM (to 0.001, as Bowditch's l and p come from five-place trigonometry).
