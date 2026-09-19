<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Traverse closure (`survey.cogo.traverse-closure`)

## Method

Latitudes and departures. Each course is split into its north (latitude) and east (departure) components. On a closed loop they should sum to zero, and the leftover is the linear misclosure. Precision is the total length over the misclosure, written 1:N. An optional adjustment distributes the misclosure: the compass (Bowditch) rule in proportion to course length, or the transit rule in proportion to each component's size. The adjusted loop closes exactly.

## Equations

- Latitude = L cos α, departure = L sin α, with α the azimuth from north.
- Misclosure e = √(Σlat² + Σdep²); its bearing is the direction of (Σlat, Σdep).
- Precision = 1 : ΣL / e.
- Compass rule: Δlat_i = −Σlat × L_i / ΣL, and likewise for departures.
- Transit rule: Δlat_i = −Σlat × |lat_i| / Σ|lat|, and likewise for departures.

## Symbols and units

L course length and α azimuth (bearings like `N 45°30'15" E` or `S44-30-00W` are parsed to azimuths), lat and dep in the length unit of the courses. Mixing US survey feet and international feet is refused.

## Domain

Two or more courses with positive lengths. A loop that closes exactly returns "perfect" with the PERFECT_CLOSURE warning, because field data rarely closes exactly.

## Approximations

Plane surveying: courses are treated as straight lines on a flat plane, which is the convention for traverse computation. The adjustment rules are conventions, not least squares, and the result names the one used.

## Worked example

- sourcePublisher: University of Memphis, Department of Civil Engineering
- sourceTitle: CIVL 1112 Surveying - Traverse Calculations (course notes)
- sourceEdition: retrieved 2026-09-19
- sourceLocator: Latitudes and departures example (courses S 6°15' W 189.53 ft through N 42°59' E 234.58 ft), error of closure, precision, and the compass-rule balanced table; group example problem 1
- independent: yes
- inputs: five courses, 939.46 ft in total, compass rule from (0, 0)
- outputs: misclosure 0.182 ft, precision 1:5,175, and balanced points B (−188.388, −20.601) through E (−171.628, −159.974); the group example gives 1.262 ft and 1:2,083
- tolerance: the printed rounding (0.0015 ft on coordinates, exact precision strings)
- verifiedBy: golden vectors v021 and v022, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_survey.py`: a separate Python implementation of latitudes, departures, and both rules (Ghilani and Wolf) at 19 loops of 3 to 6 courses (within 1e-9 relative)
- `core/vectors/survey.cogo.traverse-closure.jsonl`: those vectors plus the two published examples, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `traverse_closure_invariants`: misclosure is the length of (Σlat, Σdep), rotating every course leaves misclosure and precision unchanged, reversing the loop negates the sums, and both adjustments return exactly to the start
