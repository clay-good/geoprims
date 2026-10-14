<!-- Derivation note (trust/correctness-program layer A). -->
# Vertical curve elevations by station (`survey.curves.vertical-curve-elevations`)

## Method

An equal-tangent vertical curve is a parabola from the PVC to the PVT. Its elevation at any distance past the PVC follows from the PVC elevation, the incoming grade, and the constant rate at which the grade changes. The operation tabulates the elevation and grade at the PVC, at each even multiple of the interval inside the curve, and at the PVT, or at one requested station. It shares one calculation path with `survey.curves.vertical-curve`.

## Equations

With grades as ratios and x measured from the PVC:

- PVC station = PVI station − L / 2; PVC elevation = PVI elevation − g1·L / 2
- elevation(x) = PVC elevation + g1·x + (g2 − g1)·x² / (2L)
- grade(x) = g1 + (g2 − g1)·x / L

## Symbols and units

g1, g2 grades in percent, up positive; L curve length; the interval, length, and elevation share one unit. Stations are written 12+34.56 in feet and 1+234.567 in meters. The default interval is one full station: 100 ft, or 20 m.

## Domain

A positive length (or K value) and two different grades. The interval must be positive and give no more than 10,000 rows. A single station must lie from the PVC to the PVT; one outside is refused with OUT_OF_DOMAIN. Give an interval or one station, not both.

## Approximations

None for a parabolic curve. Stations are rounded to 0.01 ft or 0.001 m for display; elevations are computed at the unrounded distance.

## Worked example

- sourcePublisher: geoprims
- sourceTitle: add-survey-suite specification
- sourceEdition: 2026
- sourceLocator: survey/alignment-curves, requirement "Vertical parabolic curves" (elevations at any station or interval) and scenario "Crest curve high point"; the arithmetic is checked by hand, not taken from a published table
- independent: no
- inputs: g1 +2%, g2 −3%, L 600 ft, PVI 10+00 at 100.00 ft, full stations
- outputs: 7+00 at 94.00 ft (PVC), 10+00 at 96.25 ft (the PVI elevation less L·|A| / 800 = 3.75 ft), 13+00 at 91.00 ft (PVT); at station 9+40, 96.40 ft with a 0% grade
- tolerance: 1e-9 ft
- verifiedBy: golden vectors v002 and v003 and the `named_curve_operations_match_their_general_tools` integration test
- verifiedOn: 2026-10-13

## Differential tests

- `tools/vectors/gen_survey.py` `vertical_curve_parts`: 17 random curves in all three length units, with intervals from 10 to 100, default intervals, and single stations, evaluated in exact fractions; the first, middle, and last rows of each table are checked.
- `core/vectors/survey.curves.vertical-curve-elevations.jsonl`: those vectors, three hand-checked cases, and four refusal cases.
- The integration suite requires the first and last rows to match the PVC and PVT of `survey.curves.vertical-curve`.

## Invariants

- The first row is the PVC and the last is the PVT.
- The grade runs linearly from g1 to g2.
- Second differences of elevation at a constant interval are constant.
