---
title: How far can you see a drone? VLOS explained
description: Visual line of sight means seeing your drone with your own eyes all flight. What Part 107 requires, and how far away a small drone stays visible.
summary: What "visual line of sight" means under Part 107, and a published method for how far out a drone of a given size stays in sight.
audience: Drone operators
published: 2026-09-23
tools:
  - drone.sensors.vlos
  - drone.ops.part107-altitude
sources:
  - title: 14 CFR 107.31, Visual line of sight aircraft operation
    issuer: Federal Aviation Administration, via the Electronic Code of Federal Regulations
    edition: Current text, checked September 23, 2026
    locator: Paragraphs (a) and (b)
    url: https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-107/subpart-B/section-107.31
  - title: 14 CFR 107.51, Operating limitations for small unmanned aircraft
    issuer: Federal Aviation Administration, via the Electronic Code of Federal Regulations
    edition: Current text, checked September 23, 2026
    locator: Paragraph (c), minimum flight visibility
    url: https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-107/subpart-B/section-107.51
  - title: Guidelines for UAS operations in the open and specific category
    issuer: European Union Aviation Safety Agency
    edition: Issue 03, July 17, 2025
    locator: Part A, chapter I, calculation of the VLOS distance (ALOS, DLOS)
    url: https://www.easa.europa.eu/en/downloads/139435/en
---

Visual line of sight (VLOS) means you can see your drone with your own eyes, or with only glasses or contact lenses, for the whole flight. Under Part 107 you must see it well enough to know where it is, which way it is pointing and moving, and what else is in the sky around it. The rule sets no distance in feet or meters. How far that is depends on the drone's size and the day's visibility, and for a small quadcopter it can be closer than you might expect.

## What the rule says

Section 107.31 requires that the remote pilot in command, the visual observer if one is used, and the person on the controls be able to see the drone throughout the flight, unaided by any device other than corrective lenses. They must be able to:

1. know where the drone is;
2. tell its attitude, altitude, and direction of flight;
3. watch the airspace for other traffic and hazards; and
4. make sure it does not endanger people or property.

Either the pilot and the person on the controls, or a visual observer, must keep that ability for the whole flight. Binoculars, a camera feed, and first-person-view goggles do not count as seeing the drone.

A separate rule, 107.51(c), requires flight visibility of at least 3 statute miles from the control station.

## Why it matters

Seeing a dot in the sky is not the same as seeing which way it is facing. If you cannot tell the drone's orientation, you cannot steer it away from a tree or an aircraft with confidence. A mission planned 400 m out may be legal on paper and still take the drone beyond the point where you can really fly it by eye.

## How it is worked out

The rule has no formula, so the [visual line of sight tool](/drone/sensors/vlos/) uses a published one from the European Union Aviation Safety Agency (EASA). EASA's guidance sets the VLOS distance as the smaller of two limits:

- **Attitude line of sight (ALOS).** The farthest you can tell the drone's position and orientation, from its size. For multirotors, ALOS = 327 × CD + 20 m. For fixed-wing aircraft, ALOS = 490 × CD + 30 m. CD is the characteristic dimension: the drone's largest dimension, in meters.
- **Detection line of sight (DLOS).** The distance at which you can spot other aircraft in time to avoid them. DLOS = 0.3 × ground visibility.

The VLOS distance is the smaller of the two. EASA measures it as the straight-line distance between the pilot and the drone, so height counts as well as distance across the ground.

## A worked example

A multirotor 0.35 m across, on a mission whose farthest point is 400 m from the pilot:

| Step | Result |
|---|---|
| Attitude line of sight (ALOS) | 134 m |
| Detection line of sight (DLOS), visibility taken as 5 km | 1,500 m |
| VLOS distance, the smaller of the two | **134 m** |
| Farthest planned point | 400 m |
| Result | Beyond VLOS guidance by 266 m |

For this drone, size is the limit, not the weather. With the 3 statute mile visibility Part 107 requires, DLOS is 1,448 m, and the answer is still 134 m.

How the distance grows with size, in good visibility:

| Aircraft | Largest dimension | VLOS distance |
|---|---|---|
| Multirotor | 0.2 m | 85 m |
| Multirotor | 0.35 m | 134 m |
| Multirotor | 0.5 m | 184 m |
| Multirotor | 0.9 m | 314 m |
| Fixed wing | 1.2 m | 618 m |
| Fixed wing | 2 m | 1,010 m |

Visibility takes over only when it is poor. At 0.3 km ground visibility, the same 0.35 m drone's DLOS is 90 m, and that becomes the limit.

## The model's limits

- **It is guidance, not a guarantee.** EASA offers it as one acceptable method. Part 107 sets no numeric distance, and the tool labels its answer that way.
- **It ignores color, contrast, and light.** A dark drone against trees, or a drone between you and a low sun, is harder to see than the formula assumes. Bright lights and a plain sky help.
- **It knows nothing about your eyes.** Eyesight, fatigue, and glare vary from pilot to pilot and from hour to hour.
- **Visibility is capped.** EASA's guidance says ground visibility should be at least 5 km. The tool takes 5 km as the most it will use, so DLOS never goes above 1,500 m, even on a very clear day.

Treat the answer as a planning check: if your mission goes past it, move the launch point, add a visual observer, or shrink the area.

## Common mistakes

- **Planning to the edge of the map.** A mission planner will happily send a drone 1 km away. The rule is about what you can see.
- **Counting the camera feed.** A live video picture is not visual line of sight.
- **Measuring across the ground only.** The distance that matters is the straight line to the drone, including its height.
- **Measuring the drone folded.** Use its largest dimension as it flies, with the arms unfolded.
- **Forgetting the observer's view.** A visual observer must be able to see the drone too, from where they stand.

## Where the numbers come from

The requirement is 14 CFR 107.31, and the visibility minimum is 107.51(c), both checked in the current eCFR on September 23, 2026. The distance method is from EASA's guidelines for the open and specific categories, Issue 03. The [visual line of sight tool](/drone/sensors/vlos/) shows ALOS, DLOS, and your margin with each answer. For the height side of the flight, see [the Part 107 altitude limit](/learn/part-107-altitude-limit/) and the [Part 107 altitude tool](/drone/ops/part107-altitude/).
