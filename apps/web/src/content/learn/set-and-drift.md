---
title: Set and drift
description: How a current moves a vessel off its course, the three current-triangle problems Bowditch teaches, and how to find the course to steer so that you make good the track you want.
summary: Set, drift, the current triangle, and the course to steer to cancel a current.
audience: Mariners
published: 2026-10-20
tools:
  - navigation.piloting.set-and-drift
  - navigation.piloting.dead-reckoning
  - aviation.wind.heading-groundspeed
sources:
  - title: The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume I
    issuer: National Geospatial-Intelligence Agency
    edition: 2024 edition
    locator: Chapter 10, Dead Reckoning, §1008, estimated course and speed made good, examples 1 to 3
    url: https://msi.nga.mil/Publications/APN
  - title: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
    issuer: Federal Aviation Administration
    edition: 2023
    locator: Chapter 16, the wind triangle, the same vector solution in the air
    url: https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/phak
---

A vessel moves through the water, and the water moves too. Where you actually go is the sum of the two. Navigators call the water's motion the current, and they describe it with two words: **set**, the direction it flows toward, and **drift**, its speed.

Get them backward and the plot goes wrong fast. Set is where the current goes, not where it comes from. That is the opposite of wind, which is named for where it comes from.

## The current triangle

Draw three arrows head to tail:

- The **water vector**: your heading and your speed through the water.
- The **current vector**: the set and drift.
- The **ground vector**: what you actually make good over the bottom, the track and the speed made good.

The water vector plus the current vector equals the ground vector. That is the whole idea. Bowditch asks three questions of it, depending on which arrow you do not know.

1. **Where am I going?** You know your heading, speed, and the current. Add them to get the track and speed made good.
2. **What course do I steer?** You know the track you want and your speed through the water. Find the heading that, with the current, lands on that track.
3. **What course and speed do I use?** You know the track and the speed you need to make good, perhaps to arrive on time. Take the current away from that ground vector, and what is left is the water vector you must sail.

Where do set and drift come from? Usually from your own plot. Compare a fix with the dead-reckoning position for the same moment: the direction from the dead-reckoning position to the fix is the set, and the distance between them divided by the hours since the last fix is the drift.

If this looks familiar from flying, it is. The wind triangle pilots solve is the same vector sum, with the wind named for where it comes from. The tools here share one solution for both, so the two cannot disagree.

## How it is worked out

Treat each vector as east and north parts.

- East = speed × sin(direction); north = speed × cos(direction)
- Ground = water + current, one part at a time
- Turn the sums back into a direction and a speed

The course to steer takes one more step. The current pushes you sideways off the track by drift × sin(the angle between the set and the track). To cancel it, turn into the current by the angle δ whose sine is that sideways push divided by your speed through the water. If the current is faster than you are across the track, no heading works.

## A worked example

These are Bowditch's first two examples from §1008.

**Example 1, where am I going?** You steer 080° at 10 knots. The current sets 140° at 2 knots.

- Water: 9.85 knots east, 1.74 north
- Current: 1.29 east, 1.53 south
- Sum: 11.13 east, 0.20 north
- **Track 088.9°, speed made good 11.1 knots**

The current is on the starboard quarter, pushing you a little south and speeding you up.

**Example 2, what course do I steer?** You want to make good 095°, with 12 knots through the water, in a current setting 170° at 2.5 knots.

- The current pushes you south of the track, so turn north into it, by 11.6°
- **Steer 083.4°, and make good 12.4 knots**

Bowditch solves these with a plotting sheet and dividers, so his answers carry a plot's precision: 089° and 11.2 knots for the first, 083.5° for the second. The arithmetic agrees to within a tenth, and the tool gives the exact figures.

A current is rarely steady. Tidal currents turn every few hours, and an estimate of set and drift from a fix an hour ago is a guess about the next hour. Check it with each new fix.

The [set and drift tool](/navigation/piloting/set-and-drift/) solves all three problems, the [dead reckoning tool](/navigation/piloting/dead-reckoning/) carries a position forward from course and speed, and the [wind triangle](/aviation/wind/heading-groundspeed/) is the same solution for pilots.
