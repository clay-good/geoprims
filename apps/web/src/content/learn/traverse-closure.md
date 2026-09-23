---
title: Traverse closure and precision explained
description: How to check a closed traverse, from angular misclosure to linear misclosure and the 1:N precision ratio, and how the compass and transit rules adjust it.
summary: Angular and linear misclosure, the precision ratio, and how the compass and transit rules spread the error.
audience: Surveyors
published: 2026-09-23
tools:
  - survey.cogo.traverse-closure
  - survey.cogo.angular-closure
  - survey.cogo.inverse
  - survey.cogo.area-by-coordinates
  - survey.cogo.least-squares-2d
sources:
  - title: Standards and Specifications for Geodetic Control Networks
    issuer: Federal Geodetic Control Committee
    edition: September 1984
    locator: Section 3.3, Traverse (office procedures, azimuth and position closure)
    url: https://geodesy.noaa.gov/FGCS/tech_pub/1984-stds-specs-geodetic-control-networks.pdf
  - title: "Elementary Surveying: An Introduction to Geomatics"
    issuer: Ghilani, C. D., and Wolf, P. R., Pearson
    edition: 16th edition
    locator: Chapter 10, traverse computations (angular misclosure, latitudes and departures, compass and transit rules)
    url: https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148
---

A closed traverse is a chain of measured angles and distances that returns to its starting point, or ends on another known point. Because every measurement has some error, the computed end point never lands exactly on the start. The gap is the linear misclosure, and dividing the traverse length by it gives the precision ratio, written 1:N. A traverse of 1,400 ft that misses by 0.12 ft has a precision of about 1:11,500.

Closure is the first check on field work. It tells you whether the measurements hang together well enough to adjust, or whether something was misread, mistyped, or measured to the wrong point.

## Why it matters

A traverse carries control around a site: boundary corners, construction control, or the frame for a topographic survey. A bad angle or distance in the middle of it moves every point after it. Closure is how you catch that before anyone stakes a building or sets a monument. Contracts, state minimum standards, and agency specifications often state the closure a survey must meet.

## How it is worked out

There are two checks, then the adjustment, done in order.

**1. Angular misclosure.** The interior angles of a closed polygon with n sides must add to (n − 2) × 180°. The measured sum minus that value is the angular misclosure. It is compared with an allowable value, often written K√n, where K is a number of seconds set by the job's standard. If it passes, the misclosure is spread evenly: each angle gets the same small correction. The [angular closure tool](/survey/cogo/angular-closure/) does this step.

**2. Linear misclosure.** Using the balanced angles, work out each course's bearing. Each course then splits into a **latitude** (its north-south part, distance × cos bearing) and a **departure** (its east-west part, distance × sin bearing). Around a closed loop, both should add to zero. What is left over gives the misclosure:

- Linear misclosure = √(Σlatitudes² + Σdepartures²)
- Precision ratio = total length / linear misclosure, written 1:N

**3. Adjustment.** If the precision is good enough, the misclosure is spread around the traverse so it closes exactly. Two classic rules do this:

- **Compass (Bowditch) rule.** Each course's correction is in proportion to its length. It suits work where angles and distances are about equally precise, which is the common case with a total station.
- **Transit rule.** Each latitude is corrected in proportion to the size of that latitude, and each departure in proportion to the size of that departure. It was meant for work where angles are measured better than distances. Its result depends on which way the grid axes point, so rotating the grid changes the answer.

A least-squares adjustment is the rigorous option. It weights each measurement by its precision and reports how good the adjusted points are. The [least-squares tool](/survey/cogo/least-squares-2d/) handles small networks.

## A worked example

A four-sided loop with courses due north 300.00 ft, due east 400.02 ft, due south 299.95 ft, and 270.01° (just north of due west) 400.00 ft:

| Step | Result |
|---|---|
| Sum of latitudes | 0.1198 ft |
| Sum of departures | 0.02 ft |
| Linear misclosure | **0.121 ft** |
| Direction of the misclosure | N 9°28'47" E |
| Total length | 1,399.97 ft |
| Precision | **1:11,525** |

The computed end point lands 0.121 ft from the start, mostly to the north. Starting from 5,000.000 N and 5,000.000 E, the two rules place the corners like this:

| Point | Compass rule (N, E) | Transit rule (N, E) |
|---|---|---|
| 2 | 5,299.974, 4,999.996 | 5,299.940, 5,000.000 |
| 3 | 5,299.940, 5,400.010 | 5,299.940, 5,400.010 |
| 4 | 4,999.964, 5,400.006 | 4,999.930, 5,400.010 |

The transit rule keeps the north-south courses running exactly north and south, because those courses have no departure to correct. The compass rule turns every course slightly, by 3″ to 18″ here. The two answers differ by up to 0.034 ft, which is the kind of choice a surveyor should record.

For the angles, a five-sided traverse whose interior angles add to 540°00'25" misses the required 540° by 25″. With K = 10″, the allowable is 10″ × √5 = 22.4″, so that closure fails and the angles should be checked before going further.

## Typical standards

The Federal Geodetic Control Committee's 1984 standards for geodetic control traverse give these position closures, after the azimuths are adjusted:

| Order and class | Closure |
|---|---|
| First order | 1:100,000 |
| Second order, class I | 1:50,000 |
| Second order, class II | 1:20,000 |
| Third order, class I | 1:10,000 |
| Third order, class II | 1:5,000 |

The same table sets the azimuth closure for third order, class I at 10″√N, with N the number of segments. For long lines it also gives a closure that grows with the square root of the distance; use whichever is smaller. The example above, at 1:11,525, would meet the third order, class I ratio. These are standards for geodetic control. Boundary and construction work usually follow state minimum standards, the ALTA/NSPS standards, or the project's own specifications, so check which one applies.

## Common mistakes

- **Adjusting a blunder.** The compass and transit rules spread any error, even a mistyped bearing, across every course. A large misclosure means find the mistake first. If the misclosure runs nearly parallel to one course, check that course's distance first.
- **Skipping the angle check.** Unbalanced angles rotate everything after them, and the linear closure alone may not show where.
- **Treating closure as accuracy.** The FGCC standards warn not to confuse closure with the accuracy of the survey. A traverse can close perfectly and still be wrong, for example with a constant error in every distance or a wrong starting azimuth.
- **Mixing interior and exterior angles.** Exterior angles of a closed loop add to (n + 2) × 180°.
- **Forgetting grid versus ground.** If distances are ground distances and the control is state plane, reduce the distances first. See [grid vs ground distance](/learn/grid-vs-ground/).

## Where the numbers come from

Latitudes, departures, and the compass and transit rules follow Ghilani and Wolf's *Elementary Surveying*, chapter 10. The closure table is from the FGCC's 1984 *Standards and Specifications for Geodetic Control Networks*, section 3.3. The [traverse closure tool](/survey/cogo/traverse-closure/) computes the misclosure and adjusts by the compass, transit, or Crandall rule, with each step shown under "How we got this." To get a course's bearing and distance from two coordinates, use the [inverse tool](/survey/cogo/inverse/), and to find the area of the adjusted figure, the [area by coordinates tool](/survey/cogo/area-by-coordinates/).
