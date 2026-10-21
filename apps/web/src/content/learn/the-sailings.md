---
title: The sailings
description: How navigators turn a course and distance into a new position, and back, with Bowditch's six sailings, from flat-chart plane sailing to the great circle, and when each one is good enough.
summary: Plane, traverse, parallel, mid-latitude, Mercator, and great-circle sailing, with Bowditch's own examples.
audience: Mariners
published: 2026-10-20
tools:
  - navigation.sailing.mid-latitude
  - navigation.sailing.plane
  - navigation.sailing.mercator
  - navigation.sailing.great-circle
sources:
  - title: The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume I
    issuer: National Geospatial-Intelligence Agency
    edition: 2024 edition
    locator: Chapter 13, The Sailings, §1301 to §1308
    url: https://msi.nga.mil/Publications/APN
  - title: The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume II
    issuer: National Geospatial-Intelligence Agency
    edition: 2024 edition
    locator: Table 6, meridional parts
    url: https://msi.nga.mil/Publications/APN
---

"The sailings" are the old navigator's toolkit for one question: if I steer this course for this many miles, where do I end up? And the reverse: what course and distance take me from here to there?

There are six of them in Bowditch, and they differ in one thing only: how much of the Earth's roundness they bother to account for. The trick is knowing when the cheap one is good enough.

## Two words first

Every sailing works with two short distances.

- **Difference of latitude (l)**: how far north or south you went, in minutes of latitude. One minute of latitude is one nautical mile.
- **Departure (p)**: how far east or west you went, in nautical miles.

Departure is not the same as the change in longitude. Meridians crowd together toward the poles, so a mile east covers more longitude in Maine than in Panama. Converting departure to difference of longitude (DLo) is where the Earth's shape gets in.

## The six sailings

**Plane sailing** treats the run as a flat right triangle: the distance is the long side, l and p are the short sides. It is exact for l and p and fine for short runs. Bowditch's example: 188.0 miles on course 005° gives l = 187.3' north and p = 16.4 miles east.

**Traverse sailing** is plane sailing for several legs: add up all the l's and p's, then turn the totals into one course and distance made good. Bowditch's five-leg example nets out at 192.3°, 67.3 miles.

**Parallel sailing** is for running due east or west. On a parallel, DLo = p ÷ cos(latitude). At 49°30' N, a DLo of 3°30' is only 136.4 miles of departure.

**Mid-latitude sailing** combines the two: plane sailing for l and p, then parallel sailing at the latitude halfway between start and end to get DLo. It is the everyday method for runs of a few hundred miles.

**Mercator sailing** replaces the middle latitude with meridional parts, the stretched latitude scale of a Mercator chart (Bowditch Volume II, Table 6). It holds up over long distances and high latitudes, where the middle-latitude shortcut drifts.

**Great-circle sailing** is the shortest path on the globe. The course changes along the way, so it gives an initial course, a final course, and the vertex, the point nearest a pole. Bowditch's example from 22°S 116°E, off Australia, to 20°S 31°E, off Africa, is 4,693.5 miles, starting on 253°.

Every sailing but the great circle follows a rhumb line, a track of constant course. A rhumb line is easy to steer and always longer than the great circle, except along a meridian or the equator.

## How it is worked out

For mid-latitude sailing, the formulas fit in four lines:

- l = D cos C
- p = D sin C
- Lm = (L1 + L2) ÷ 2
- DLo = p ÷ cos Lm

Here D is the distance, C the course, and L1 and L2 the two latitudes. Going the other way, from two positions to a course and distance, run them backward: p = DLo × cos Lm, then C = atan(p ÷ l) and D = √(l² + p²).

## A worked example

Bowditch's mid-latitude example starts at 15°17.0'N 151°37.0'E and runs 1,253 miles on 070°.

- l = 1,253 × cos 70° = 428.6' north, so the new latitude is 22°25.6'N
- Lm = 18°51.3'N
- p = 1,253 × sin 70° = 1,177.4 miles east
- DLo = 1,177.4 ÷ cos 18°51.3' = 1,244.2' east, or 20°44.2'
- **Arrival: 22°25.6'N 172°21.2'E**

The book prints 172°21.1'E, because it rounds DLo to 1,244.1' along the way. Hand methods round; the tool does not, and says where the book's last digit comes from.

On a run that long, the sailings and the true shape of the Earth start to disagree. The mid-latitude and Mercator tools show the exact rhumb line on the WGS 84 ellipsoid beside their answers, and the great-circle tool the ellipsoid's geodesic, so you can see how much the sphere costs you.

The [mid-latitude sailing tool](/navigation/sailing/mid-latitude/) works this example, the [plane sailing tool](/navigation/sailing/plane/) handles short legs, the [Mercator sailing tool](/navigation/sailing/mercator/) uses meridional parts, and the [great-circle sailing tool](/navigation/sailing/great-circle/) finds the shortest route and its vertex.
