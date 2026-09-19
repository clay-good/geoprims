<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Combined grid-to-ground factor (`survey.reduction.combined-factor`)

## Method

The NGS reduction from a horizontal ground distance to a state plane grid distance. The elevation factor carries the distance from its height down to the ellipsoid, and the grid scale factor carries it from the ellipsoid to the projection. Their product is the combined factor. Grid distance = ground × combined, and ground = grid / combined.

## Equations

- Elevation factor = R / (R + h), with h = H + N when an orthometric height H and geoid height N are given.
- Combined factor = k × elevation factor.
- Grid = ground × combined, ground = grid / combined.

## Symbols and units

k grid scale factor (unitless), h ellipsoid height, H orthometric elevation, N geoid height, R mean Earth radius (default 6,372,000 m = 20,906,000 ft, per NGS Manual 5). Distances in any length unit. US survey and international feet are kept apart.

## Domain

Grid scale factors from 0.9 to 1.1 and a radius from 1,000 km to 10,000 km. Heights are not limited beyond the shared input bounds. When only an elevation is given, it is used as the ellipsoid height, with the ORTHOMETRIC_AS_ELLIPSOIDAL warning, since that makes the factor too small by up to about 5 ppm in the conterminous US.

## Approximations

A mean radius instead of the radius of curvature along the line is good to about 1 ppm (NGS Manual 5). A local radius can be supplied. A single combined factor for a whole line uses the mean height and mean scale factor.

## Worked example

- sourcePublisher: National Oceanic and Atmospheric Administration, National Geodetic Survey
- sourceTitle: NOAA Manual NOS NGS 5, State Plane Coordinate System of 1983 (James E. Stem)
- sourceEdition: 1990 (reprint of 1989)
- sourceLocator: Section 4.4 example, steps 2 to 5 (mean grid scale factor 1.0000450, mean H 865 ft, mean N −100 ft)
- independent: yes
- inputs: grid scale 1.0000450, elevation 865 ft, geoid height −100 ft, radius 20,906,000 ft, ground distance 4,805.468 ft
- outputs: elevation factor 0.9999634, combined factor 1.0000084, grid distance 4,805.508 ft (and the four other lines of step 5)
- tolerance: the printed digits (5e-8 on factors, 0.0005 ft on distances)
- verifiedBy: golden vectors v018 to v022, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_survey.py`: a separate Python evaluation of the NGS Manual 5 relations at 16 cases, with ellipsoid heights from −200 ft to 14,000 ft and with orthometric plus geoid heights (within 1e-12 relative)
- `core/vectors/survey.reduction.combined-factor.jsonl`: those vectors plus the five published lines, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `combined_factor_invariants`: combined = k × elevation factor, grid and ground distances invert each other, H + N gives the same factor as the equivalent ellipsoid height, and the elevation factor falls as height rises
