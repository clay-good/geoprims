---
title: Horizontal curves explained
description: The parts of a circular road curve (R, Δ, T, L, LC, E, M, and degree of curve), how to station the PC and PT, and how to stake it by deflection angles.
summary: The parts of a circular curve, arc versus chord degree of curve, stationing, and deflection-angle layout.
audience: Surveyors
published: 2026-09-23
tools:
  - survey.curves.circular-curve
  - survey.curves.curve-layout
  - survey.curves.spiral
  - survey.cogo.station-offset
sources:
  - title: Highway Surveying Manual (M 22-97), Chapter 11, Geometrics
    issuer: Washington State Department of Transportation
    edition: January 2005
    locator: Circular curves, nomenclature, and degree of curvature (arc and chord definitions), pages 11-1 to 11-8
    url: https://wsdot.wa.gov/publications/manuals/fulltext/m22-97/chapter11.pdf
  - title: MDT Survey Manual, Appendix C, Curves
    issuer: Montana Department of Transportation
    edition: Current web edition
    locator: Circular curve, general formulas for arc definition, locating the PC and PT, and simple curve computation by deflection angle, pages C-1 to C-3
    url: https://www.mdt.mt.gov/other/webdata/external/photosurvey/survey/manual_guides_forms/survey_manual/Survey-Manual-Appendix-C.pdf
  - title: "Elementary Surveying: An Introduction to Geomatics"
    issuer: Ghilani, C. D., and Wolf, P. R., Pearson
    edition: 16th edition
    locator: Chapter 24, horizontal curves
    url: https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148
---

A horizontal curve is the circular arc that joins two straight sections of a road, railroad, or property line. It is fixed by two numbers: its radius R and its deflection angle Δ, the angle by which the second tangent turns from the first. Every other part of the curve, from the tangent length to the stationing of its ends, follows from those two.

Surveyors use these parts to stake the curve in the field, to check a plan or a deed call, and to compute coordinates along an alignment.

## The parts of a curve

The two straight lines, extended, meet at the **PI**, the point of intersection. The curve leaves the back tangent at the **PC**, the point of curvature, and joins the forward tangent at the **PT**, the point of tangency.

| Symbol | Name | Formula |
|---|---|---|
| T | Tangent distance, PI to PC (and PI to PT) | R tan(Δ/2) |
| L | Length of curve, along the arc | R Δ, with Δ in radians |
| LC | Long chord, PC to PT in a straight line | 2R sin(Δ/2) |
| E | External distance, PI to the middle of the curve | R (sec(Δ/2) − 1) |
| M | Middle ordinate, middle of the chord to the middle of the curve | R (1 − cos(Δ/2)) |

**Degree of curve.** Before radius became the usual way to describe a curve, sharpness was given as a degree of curve D. There are two definitions, and they differ:

- **Arc definition,** used in highway work: D is the central angle for 100 ft of arc. D = 5,729.578 / R, with R in feet.
- **Chord definition,** used in railroad work and by some counties: D is the central angle for a 100 ft chord. sin(D/2) = 50 / R.

WSDOT's survey manual gives the radius of a 1° curve as 5,729.578 ft by the arc definition and 5,729.65 ft by the chord definition. Degree of curve is not used in metric work.

## Stationing the PC and PT

Stations run along the alignment, with 12+34.56 meaning 1,234.56 ft from the start. The PC is found by going back from the PI along the tangent. The PT is found by going forward from the PC along the arc:

- PC station = PI station − T
- PT station = PC station + L

## A worked example

A curve with a 500 ft radius, a 30° deflection, and its PI at station 12+34.56:

| Part | Result |
|---|---|
| Tangent T | 133.975 ft |
| Length L | 261.799 ft |
| Long chord LC | 258.819 ft |
| External E | 17.638 ft |
| Middle ordinate M | 17.037 ft |
| Degree of curve, arc | 11.4592° |
| Degree of curve, chord | 11.4783° |
| PC station | **11+00.59** |
| PT station | **13+62.38** |

The PT station is 127.82 ft past the PI station, not the 133.975 ft tangent length, because stations run along the arc, which is shorter than the two tangents it cuts across.

## Staking by deflection angles

The classic way to lay out a curve is to set up at the PC, sight along the back tangent, and turn a deflection angle to each station. The deflection angle to a point is half the central angle to it, which for arc length s is s / (2R) in radians. The chord from the PC to the point is 2R sin(deflection).

Staked every 50 ft on full stations, the same curve gives:

| Station | Deflection from the back tangent | Chord from the PC |
|---|---|---|
| 11+00.59 (PC) | 0°00'00.0" | 0 ft |
| 11+50.00 | 2°49'52.5" | 49.394 ft |
| 12+00.00 | 5°41'45.7" | 99.251 ft |
| 12+50.00 | 8°33'39.0" | 148.859 ft |
| 13+00.00 | 11°25'32.2" | 198.096 ft |
| 13+50.00 | 14°17'25.5" | 246.837 ft |
| 13+62.38 (PT) | 15°00'00.0" | 258.819 ft |

The first and last chords are short, because the PC and PT fall between full stations. The deflection to the PT must equal Δ/2, here 15°, which is the built-in check the MDT manual calls for.

## Rules of thumb, and how far they drift

**A chord is almost as long as its arc.** On this curve, a full 50 ft of arc has a chord of 49.979 ft, only 0.021 ft shorter. Taping 50 ft instead of the chord would put a stake about a quarter inch out, and if you chain from stake to stake the error adds up. On a sharper curve or a longer interval it is larger.

**Arc and chord degree are close on flat curves, not on sharp ones.** For a 1° curve the two radii differ by 0.07 ft. For this 500 ft curve, D is 11.4592° by arc and 11.4783° by chord, and a curve called "11.4592°" in the wrong definition would have the wrong radius.

## Common mistakes

- **Stationing the PT from the PI.** PT = PC + L, never PI + T.
- **Mixing arc and chord definitions.** Check which one the plans, deed, or railroad use before turning D into a radius.
- **Using Δ in degrees in L = RΔ.** It needs radians, or use L = 100Δ/D with the arc definition.
- **Setting chords without the subchord at each end.** The first chord runs from the PC to the next full station.
- **Forgetting the spiral.** Many highway curves have spiral transitions, and then the circular curve starts at the SC, not at a PC on the tangent. The [spiral tool](/survey/curves/spiral/) handles that case.

## Where the numbers come from

The curve formulas and both definitions of degree of curve are in WSDOT's *Highway Surveying Manual*, chapter 11, and the MDT *Survey Manual*, appendix C, which also gives the deflection-angle method. Ghilani and Wolf's *Elementary Surveying*, chapter 24, covers the same ground. The [circular curve tool](/survey/curves/circular-curve/) works out every part from any two, and the [curve layout tool](/survey/curves/curve-layout/) builds the staking table at any interval. To find a point's station and offset from a straight tangent, use the [station and offset tool](/survey/cogo/station-offset/). For the profile of the same road, see [vertical curves explained](/learn/vertical-curves/).
