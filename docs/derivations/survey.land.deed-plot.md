<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Deed plotter (`survey.land.deed-plot`)

## Method

Plot the calls of a metes-and-bounds description as written: each line by its bearing and distance, and each curve by its chord (given, or found from the radius, delta or arc, and turn when it is tangent to the call before it). Report how far the figure misses closing, and in what direction, with the precision. The area uses an implied closing line, so the description is never adjusted unless the user chooses the compass rule, which the result labels. Curve segments add or subtract their circular segment from the chord polygon. Results are in the unit the calls use, and US survey feet and international feet are never mixed.

## Equations

- Line: ΔN = L cos α, ΔE = L sin α, with α the azimuth from the bearing.
- Misclosure = √(ΣΔN² + ΣΔE²); precision = 1 : ΣL / misclosure (two significant figures).
- Area (shoelace on the corners, closing line implied) + Σ segments, where a segment is R²(Δ − sin Δ)/2, signed by the curve's turn.
- Compass rule, when chosen: each corner moves by −(ΣΔN, ΣΔE) × (run length / ΣL).
- Acres = ft² / 43,560 (US survey acres from US survey square feet).

## Symbols and units

Bearings in quadrant form (N 45°30'15" E, S44-30-00W) or azimuths, distances in ft, ftUS, m, or other lengths, curve elements R, Δ, arc, chord, and turn. Area in the square of the calls' unit (m² for metric), plus acres.

## Domain

Two or more calls with positive lengths. Curves need enough elements to place their chords. A figure that encloses no area is refused with DEGENERATE_GEOMETRY, and mixed US survey and international feet with UNIT_MISMATCH.

## Approximations

Plane surveying, as deeds are written. The area with an implied closing line is the area the description encloses as written. Adjusting it is a choice the tool labels, because a deed's calls are evidence, not measurements to correct.

## Worked example

- sourcePublisher: University of Memphis, Department of Civil Engineering
- sourceTitle: CIVL 1112 Surveying - Traverse Calculations (course notes)
- sourceEdition: retrieved 2026-09-19
- sourceLocator: Latitudes and departures example (S 6°15' W 189.53 ft … N 42°59' E 234.58 ft): error of closure 0.182 ft, precision 1:5,175; area by double meridian distances of the balanced traverse 36,320 ft² = 0.834 acre
- independent: yes
- inputs: the five calls in feet, compass adjustment
- outputs: misclosure 0.182 ft, precision 1:5,200 (two figures of 1:5,175), area 36,320 ft², 0.834 acre
- tolerance: the printed rounding (0.0005 ft, 0.5 ft², 0.0005 acre)
- verifiedBy: golden vector v018, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_survey.py`: a separate Python implementation of latitudes, departures, and the shoelace area at 17 deeds, including 12 random quadrilaterals to hexagons with bearings to the second (within 1e-12 relative)
- `core/vectors/survey.land.deed-plot.jsonl`: those vectors, the published example, a metric deed, and the mixed-feet refusal, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/land.rs` `deed_plot_invariants`: rotating every bearing keeps the misclosure and area, calls in meters give the same numbers in meters, and the misclosure equals the traverse-closure tool's for the same courses
