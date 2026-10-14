<!-- Derivation note (trust/correctness-program layer A). -->
# Elevation factor (`survey.reduction.elevation-factor`)

## Method

The elevation factor carries a horizontal distance measured at a height down to the ellipsoid. It is the first half of the NGS reduction from ground to grid; the grid scale factor is the second half. The operation reports the factor and the ellipsoid height it used. It shares one calculation path with `survey.reduction.combined-factor`.

## Equations

- h = H + N when an orthometric elevation H and a geoid height N are given; otherwise h is the ellipsoid height as entered.
- Elevation factor = R / (R + h).

## Symbols and units

h ellipsoid height, H orthometric elevation, N geoid height, R mean Earth radius (default 6,372,000 m = 20,906,000 ft, per NGS Manual 5). Heights take any length unit and are converted to meters. The factor is unitless. The height used is reported in the unit of the first height entered.

## Domain

A radius from 1,000 km to 10,000 km. Give the ellipsoid height alone, or an elevation with or without a geoid height; an ellipsoid height together with an elevation is refused. When only an elevation is given it is used as the ellipsoid height with the ORTHOMETRIC_AS_ELLIPSOIDAL warning, since that shifts the factor by up to about 5 ppm in the conterminous US.

## Approximations

A mean radius instead of the radius of curvature along the line is good to about 1 ppm (NGS Manual 5). A local radius can be supplied. One factor for a whole line uses its mean height.

## Worked example

- sourcePublisher: National Oceanic and Atmospheric Administration, National Geodetic Survey
- sourceTitle: NOAA Manual NOS NGS 5, State Plane Coordinate System of 1983 (James E. Stem)
- sourceEdition: 1990 (reprint of 1989)
- sourceLocator: Section 4.4 example, steps 2 to 4 (mean H 865 ft, mean N −100 ft, R 20,906,000 ft)
- independent: yes
- inputs: elevation 865 ft, geoid height −100 ft, radius 20,906,000 ft
- outputs: elevation factor 0.9999634, ellipsoid height used 765 ft
- tolerance: the printed digits (5e-8 on the factor)
- verifiedBy: golden vector v002, run by the core on every build
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` `named_factor`: a separate Python evaluation of R / (R + h) at 18 cases in feet, US survey feet, and meters, with ellipsoid heights, orthometric plus geoid heights, and local radii (within 1e-11 relative).
- `core/vectors/survey.reduction.elevation-factor.jsonl`: those vectors, the published NGS example, and three refusal cases, run through the core on every build.

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `named_factor_operations_match_the_combined_factor`: the factor equals the elevation factor `survey.reduction.combined-factor` reports for the same heights, with the same warnings.
- The factor is below 1 above the ellipsoid and above 1 below it.
- H + N gives the same factor as the equivalent ellipsoid height.
