<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Vertical curve (`survey.curves.vertical-curve`)

## Method

A symmetric parabolic vertical curve centered on the PVI. The PVC and PVT lie half the curve length before and after the PVI, on the tangent grades. The curve's elevation is the PVC elevation plus the first grade times the distance, plus the parabolic offset. The high or low point is where the grade reaches zero, if that falls on the curve. K is the length per percent of grade change.

## Equations

- PVC = PVI − L/2, PVT = PVI + L/2 (stations).
- Elev(PVC) = Elev(PVI) − g1 L/2, Elev(PVT) = Elev(PVI) + g2 L/2 (grades as fractions).
- y(x) = Elev(PVC) + g1 x + (g2 − g1) x² / (2L).
- Turning point: x = −g1 L / (g2 − g1), on the curve when 0 < x < L; y there = Elev(PVC) − g1² L / (2(g2 − g1)).
- K = L / |G2 − G1| (grades in percent).

## Symbols and units

g1 and g2 the incoming and outgoing grades (percent at the interface), L the curve length, x the distance from the PVC, stations as 12+34.56. A sag has g2 > g1 (low point), and a crest has g2 < g1 (high point).

## Domain

A positive length and two different grades. When both grades have the same sign the extreme is at an end, and the tool says so instead of reporting an interior point.

## Approximations

None: the parabola is the design definition. Unsymmetric curves are not covered.

## Worked example

- sourcePublisher: Indiana Department of Transportation
- sourceTitle: Indiana Design Manual, chapter 44 (vertical alignment)
- sourceEdition: current chapter, retrieved 2026-09-19
- sourceLocator: Example 44-3.1, figure 44-3G (G1 = −1.75%, G2 = +2.25%, PVI 13+80 at 577.50, L = 500 ft)
- independent: yes
- inputs: g1 −1.75, g2 2.25, length 500 ft, PVI 13+80, elevation 577.50 ft
- outputs: PVC 11+30.00 at 581.875, PVT 16+30.00 at 583.125, low point at 13+48.75, K = 125. The example prints a low-point elevation of 580.33, but its own formula gives 581.875 − 500 × 1.75² / 800 = 579.961 (the text has 1.545 for 1.914). Its table's Z column also does not follow its stated Z = X²/25,000. Those values are not pinned, and the core gives 579.961.
- tolerance: exact stations; 1e-9 ft on elevations
- verifiedBy: golden vector v021, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `tools/vectors/gen_survey.py`: a separate Python implementation at 20 curves, crest and sag, grades from −5% to +6%, lengths 150 ft to 1,200 ft (within 1e-11 relative)
- `core/vectors/survey.curves.vertical-curve.jsonl`: those vectors and the INDOT example, run through the core on every build

## Invariants

- `core/crates/gp-survey/tests/survey.rs` `vertical_curve_invariants`: the PVC and PVT lie on the tangents, K = L/|G2 − G1|, and the reported turning point is the extreme (the curve is no lower in a sag, or higher on a crest, 1 ft either side)
