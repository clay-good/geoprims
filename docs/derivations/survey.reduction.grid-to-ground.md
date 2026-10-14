<!-- Derivation note (trust/correctness-program layer A). -->
# Grid distance to ground distance (`survey.reduction.grid-to-ground`)

## Method

The NGS reduction between a horizontal ground distance and a state plane or other grid distance, in the grid-to-ground direction. The elevation factor carries a distance between its height and the ellipsoid, and the grid scale factor carries it between the ellipsoid and the projection. Their product is the combined factor. The operation shares one calculation path with `survey.reduction.combined-factor`.

## Equations

- h = H + N when an orthometric elevation H and a geoid height N are given.
- Elevation factor = R / (R + h).
- Combined factor = k × elevation factor.
- Ground = grid ÷ combined factor.

## Symbols and units

k grid scale factor (unitless), h ellipsoid height, H orthometric elevation, N geoid height, R mean Earth radius (default 6,372,000 m = 20,906,000 ft, per NGS Manual 5). The distance keeps the unit it was entered in, including the US survey foot.

## Domain

Grid scale factors from 0.9 to 1.1 and a radius from 1,000 km to 10,000 km. The grid distance and the grid scale factor are required, with the ellipsoid height or an elevation. When only an elevation is given it is used as the ellipsoid height with the ORTHOMETRIC_AS_ELLIPSOIDAL warning.

## Approximations

A mean radius instead of the radius of curvature along the line is good to about 1 ppm (NGS Manual 5). A single combined factor for a whole line uses its mean height and mean scale factor. The ground distance is horizontal at the height given, not a slope distance.

## Worked example

- sourcePublisher: National Oceanic and Atmospheric Administration, National Geodetic Survey
- sourceTitle: NOAA Manual NOS NGS 5, State Plane Coordinate System of 1983 (James E. Stem)
- sourceEdition: 1990 (reprint of 1989)
- sourceLocator: Section 4.4 example, steps 2 to 5 (mean grid scale factor 1.0000450, mean H 865 ft, mean N −100 ft)
- independent: yes
- inputs: grid distance 4,805.508 ft, grid scale 1.0000450, elevation 865 ft, geoid height −100 ft, radius 20,906,000 ft
- outputs: elevation factor 0.9999634, combined factor 1.0000084, ground distance 4,805.468 ft (and the four other lines of step 5)
- tolerance: the printed digits (5e-8 on factors, 0.0006 ft on distances)
- verifiedBy: golden vectors v002 to v006, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` `named_factor`: a separate Python evaluation of the NGS Manual 5 relations at 15 cases in feet, US survey feet, and meters, with ellipsoid heights, orthometric plus geoid heights, and local radii (within 1e-11 relative).
- `core/vectors/survey.reduction.grid-to-ground.jsonl`: those vectors, the five published lines, and three refusal cases, run through the core on every build.

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `named_factor_operations_match_the_combined_factor`: every output equals the one `survey.reduction.combined-factor` reports for the same inputs.
- Converting with `survey.reduction.ground-to-grid` returns the starting distance.
- The combined factor equals the grid scale factor times the elevation factor.
