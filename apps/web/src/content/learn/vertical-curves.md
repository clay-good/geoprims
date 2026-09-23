---
title: Vertical curves explained
description: How an equal-tangent vertical curve joins two grades, what the K value means, and how to find the high or low point and the elevation at any station.
summary: Grades, the parabola that joins them, K value, the high or low point, and elevations along the curve.
audience: Surveyors
published: 2026-09-23
tools:
  - survey.curves.vertical-curve
  - survey.curves.unequal-vertical-curve
  - survey.curves.sight-distance
  - survey.earthwork.profile-grades
sources:
  - title: MDT Survey Manual, Appendix C, Curves
    issuer: Montana Department of Transportation
    edition: Current web edition
    locator: Symmetrical vertical curves (algebraic calculations; high or low point), page C-8
    url: https://www.mdt.mt.gov/other/webdata/external/photosurvey/survey/manual_guides_forms/survey_manual/Survey-Manual-Appendix-C.pdf
  - title: Highway Surveying Manual (M 22-97), Chapter 11, Geometrics
    issuer: Washington State Department of Transportation
    edition: January 2005
    locator: Vertical curves (crest, sag, and nonsymmetrical), pages 11-19 to 11-20
    url: https://wsdot.wa.gov/publications/manuals/fulltext/m22-97/chapter11.pdf
  - title: "Elementary Surveying: An Introduction to Geomatics"
    issuer: Ghilani, C. D., and Wolf, P. R., Pearson
    edition: 16th edition
    locator: Chapter 25, vertical curves (equal-tangent parabolic curves, sight distance)
    url: https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148
---

A vertical curve is the smooth rise or dip that joins two straight grades on a road profile. It is a parabola, not a circle, because a parabola's grade changes at a steady rate along its length, which is easy to compute and comfortable to drive. An equal-tangent vertical curve is centered on the point where the two grades meet, with half its length on each side.

A crest curve joins grades that form a hilltop. A sag curve joins grades that form a valley. The same formulas work for both.

## Why it matters

Vertical curves set the finished grade of a road, a runway, a parking lot, or a pipe. Survey crews stake them, check them against the plans, and use them to set grade on curbs and pavement. The curve also sets how far a driver can see over a hill or, at night, how far the headlights reach in a dip. WSDOT's survey manual says a vertical curve is needed when the grade changes by more than about 0.5 percent.

## The parts of a curve

| Term | Meaning |
|---|---|
| g1, g2 | The incoming and outgoing grades, in percent. Uphill is positive. |
| PVI | Point of vertical intersection, where the two grade lines meet |
| PVC | Start of the curve, L/2 before the PVI |
| PVT | End of the curve, L/2 after the PVI |
| L | Length of curve, measured horizontally |
| A | Algebraic change in grade, g2 − g1 |
| K | Length per percent of grade change, L / \|A\| |

## How it is worked out

With x the horizontal distance from the PVC, the curve's elevation is:

y = y(PVC) + g1·x + (g2 − g1)·x² / (2L)

The first two terms are the straight back grade. The last term is the offset from that grade, which grows with the square of the distance. At the PVI station, the curve is |g2 − g1|·L / 8 from the PVI elevation, with grades in percent and L in stations. The MDT survey manual calls this the middle ordinate.

**High or low point.** The grade on the curve changes steadily from g1 to g2, so it passes through zero where x = g1·L / (g1 − g2). That point is the top of a crest or the bottom of a sag. It only falls on the curve when the grades have opposite signs.

**K value.** K is the horizontal distance needed for each 1 percent change in grade. A larger K is a flatter, longer curve. Design standards set a minimum K for each design speed, for crest curves by stopping sight distance and for sag curves by headlight distance, in tables in the AASHTO Green Book and in state design manuals. With K chosen, L = K × |A|.

## A worked example

A crest curve from a +2% grade to a −3% grade, 600 ft long, with the PVI at station 10+00 and elevation 100.00 ft:

| Part | Result |
|---|---|
| PVC | 7+00.00, elevation 94 ft |
| PVT | 13+00.00, elevation 91 ft |
| K | 120 |
| High point | **9+40.00, elevation 96.4 ft** |

The high point is 240 ft past the PVC, before the PVI, because the curve starts on the gentler grade. Elevations along the curve at full stations:

| Station | Curve elevation |
|---|---|
| 7+00.00 (PVC) | 94 ft |
| 8+00.00 | 95.583 ft |
| 9+00.00 | 96.333 ft |
| 10+00.00 (under the PVI) | 96.25 ft |
| 11+00.00 | 95.333 ft |
| 12+00.00 | 93.583 ft |
| 13+00.00 (PVT) | 91 ft |

At the PVI station the curve is 3.75 ft below the PVI elevation of 100 ft, which is |g2 − g1|·L / 8 = 5 × 6 / 8.

## Sight distance

On a crest curve the hill hides the road beyond it. The minimum length for a driver to see an object over the hill depends on the sight distance, the grade change, the driver's eye height, and the object's height. With 400 ft of sight distance, A = 5%, a 3.5 ft eye height, and a 2.0 ft object, the [sight distance tool](/survey/curves/sight-distance/) gives a minimum length of 368.34 ft, a K of 73.7. The example curve, at 600 ft and K = 120, is longer than that. Take design sight distances and heights from your agency's design manual; the tool takes them as inputs rather than reproducing the Green Book's tables.

## Rules of thumb, and how far they drift

**The high point is at the PVI.** Only when the two grades are equal and opposite. In the example, with +2% and −3%, the high point is 60 ft before the PVI and 0.15 ft higher than the curve elevation there (96.4 ft against 96.25 ft). On a sag with a flat incoming grade the low point can sit near one end of the curve.

**Offsets grow with the square of distance.** Halfway from the PVC to the PVI, the offset from the back grade is a quarter of the middle ordinate. That makes it easy to check a staked elevation by hand.

## Common mistakes

- **Mixing percent and decimals.** In y = y(PVC) + g1·x + (g2 − g1)·x²/(2L), use grades as decimals with x in feet, or percent with x and L in stations. Mixing them is off by a factor of 100.
- **Measuring L along the curve.** L is horizontal, like stations.
- **Assuming the curve is centered when it is not.** When the plans show different lengths on each side of the PVI, the curve is unequal-tangent. Use the [unequal-tangent tool](/survey/curves/unequal-vertical-curve/).
- **Reporting the PVI as the high point.** The PVI is on the grade lines, above a crest curve and below a sag curve, not on the road.
- **Getting the sign of a grade wrong.** Grades are signed in the direction of stationing. A road that climbs as stations increase has a positive grade.

## Where the numbers come from

The equal-tangent formulas, the middle ordinate, and the high and low point are in the MDT *Survey Manual*, appendix C, and in WSDOT's *Highway Surveying Manual*, chapter 11. Ghilani and Wolf's *Elementary Surveying*, chapter 25, covers the same method and the sight distance cases. The [vertical curve tool](/survey/curves/vertical-curve/) finds the PVC, PVT, K, and high or low point from the grades and length, or the length from K. The [unequal-tangent tool](/survey/curves/unequal-vertical-curve/) gives the elevation at any station; with both halves set to 300 ft it is the same curve, and that is how the station table above was made. To check the grades between PVIs on an existing profile, use the [profile grades tool](/survey/earthwork/profile-grades/). For the plan view of the same road, see [horizontal curves explained](/learn/horizontal-curves/).
