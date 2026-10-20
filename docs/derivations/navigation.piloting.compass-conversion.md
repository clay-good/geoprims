<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Compass conversion (`navigation.piloting.compass-conversion`)

## Method

True − variation = magnetic; magnetic − deviation = compass, east positive, with deviation read from a deviation table at the magnetic heading by `gp_geo::compass`, the function the aviation heading chain uses. Compass to true iterates m = c + dev(m) to a fixed point. A bearing takes the ship's heading's deviation (Volume II §402).

## Equations

- M = T − V, C = M − D, compass error CE = V + D.
- From compass: M = C + D(M), T = M + V.
- Relative bearing = true bearing − true heading.

## Symbols and units

Directions in degrees; variation and deviation in degrees, east positive, or text like 7 W.

## Domain

Variation within ±180°; deviations within ±30° (a larger one needs adjusting).

## Approximations

Linear interpolation between the deviation table's rows.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency
- sourceTitle: The American Practical Navigator (Bowditch), Pub. No. 9
- sourceEdition: 2024 edition
- sourceLocator: Volume II §402, the worked example and problems 1 and 2
- independent: yes
- inputs: True heading 215°, variation 7° W, deviation 1.5° W, a light at 306.5° by compass; and problem rows
- outputs: Magnetic 222°, compass 223.5°, error 8.5° W; light 305° magnetic, 298° true, 083° relative; problem 1 row 1 magnetic 090°, compass 095°, error 10° E; problem 2 magnetic 151°, true 170°
- tolerance: exact
- verifiedBy: golden vector v001 and `core/crates/gp-navigation/tests/piloting.rs`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_piloting.py compass-conversion`: Python TVMDC chain at 16 random headings in each reference (1e-9) and the bad-variation refusal.
- `core/vectors/navigation.piloting.compass-conversion.jsonl`: those vectors, run through the core on every build.

## Invariants

- `compass_bowditch_402`: the example and three problem rows; a deviation table gives the same headings as `gp_geo::compass::true_to_compass` and converts back to true within 1e-9°.
