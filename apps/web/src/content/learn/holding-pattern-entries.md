---
title: Holding pattern entries explained
description: How to pick a direct, teardrop, or parallel holding entry from your heading, the 70° line in AIM 5-3-8, left-hand holds, and the 5° zone where either entry works.
summary: The three holding entries, how your heading at the fix picks one, and how to fly each, for right and left turns.
audience: Pilots
published: 2026-09-23
tools:
  - aviation.ifr.hold-entry
  - aviation.ifr.hold-wind-timing
  - aviation.ifr.hold-speed-limit
sources:
  - title: Aeronautical Information Manual
    issuer: Federal Aviation Administration
    edition: Current edition
    locator: Chapter 5, Section 3, paragraph 5-3-8, Holding (entry procedures, nonstandard patterns, timing)
    url: https://www.faa.gov/air_traffic/publications/atpubs/aim_html/chap5_section_3.html
  - title: Instrument Flying Handbook (FAA-H-8083-15B)
    issuer: Federal Aviation Administration
    edition: FAA-H-8083-15B
    locator: Chapter 10, IFR Flight (holding procedures, wind correction, and leg timing)
    url: https://www.faa.gov/sites/faa.gov/files/regulations_policies/handbooks_manuals/aviation/FAA-H-8083-15B.pdf
  - title: Transport Canada Aeronautical Information Manual (TP 14371E)
    issuer: Transport Canada
    edition: AIM 2026-1, effective March 19, 2026
    locator: RAC 10.2 (holding at a clearance limit, example) and RAC 10.5 (entry procedures, five-degree zone of flexibility)
    url: https://tc.canada.ca/en/aviation/publications/transport-canada-aeronautical-information-manual-tc-aim-tp-14371
---

A holding entry is the way you join a holding pattern the first time you reach the fix. There are three: direct, teardrop, and parallel. Which one to fly depends on your heading as you arrive at the fix, compared with the holding course. The *Aeronautical Information Manual* (AIM) recommends them in paragraph 5-3-8.

Each entry is a way to turn onto the holding course without leaving the protected airspace around the pattern. The AIM notes that the protected airspace is designed in part around pilots flying these entries. With RNAV guidance, it also allows the navigator to compute the entry for you.

## Why it matters

A hold is protected airspace of a set size, sized for an airplane that crosses the fix at a legal speed and flies a standard entry. A late or wrong entry, especially at a high speed, can carry you outside it. In the airplane, you also have little time: the entry has to be decided before you reach the fix, often while you copy a clearance and slow down.

## The three sectors

Draw a line through the fix at 70° to the holding course, on the holding side. The AIM calls it the 70° line. It and the holding course split the circle around the fix into three sectors. Your heading as you reach the fix says which one you arrive from.

For a standard right-turn hold, compare your heading with the inbound course:

| Your heading, relative to the inbound course | Sector size | Entry |
|---|---|---|
| From 70° left to 110° right | 180° | Direct |
| From 110° right to 180° | 70° | Teardrop |
| From 180° to 70° left | 110° | Parallel |

The two edges at 70° left and 110° right are opposite ends of the same line: the 70° line.

For a nonstandard left-turn hold, the AIM says the entries are set by the 70° line on the holding side just as for a standard hold. The table mirrors: direct from 70° right to 110° left, teardrop from 110° left to 180°, and parallel from 180° to 70° right.

## How to fly each entry

- **Direct.** Fly to the fix and turn in the direction of the hold onto the outbound leg.
- **Teardrop.** Cross the fix and fly a heading 30° off the outbound course, on the holding side, for one minute. Then turn in the direction of the hold to intercept the inbound course.
- **Parallel.** Cross the fix and turn to parallel the outbound course on the non-holding side for one minute. Then turn back toward the holding side, through more than 180°, and return to the fix or intercept the inbound course.

At or below 14,000 ft MSL, the inbound leg is one minute; above 14,000 ft it is one and a half. The first outbound leg is flown for the same time, then adjusted to make the inbound leg come out right.

## A worked example

A hold on an inbound course of 360° with right turns, and you arrive at the fix on a heading of 090°. The [holding entry tool](/aviation/ifr/hold-entry/) gives:

| Item | Result |
|---|---|
| Your heading relative to the inbound course | 90° |
| Entry | **Direct** |
| Outbound course | 180° |
| Teardrop heading, if you needed it | 150° |
| Parallel heading, if you needed it | 180° |

Its instruction: cross the fix and turn right to the outbound course, 180°.

Change only your heading, and the entry changes with it:

| Heading at the fix | Relative to inbound | Entry |
|---|---|---|
| 090° | 90° right | Direct |
| 150° | 150° right | Teardrop: fly 150° for about 1 minute, then turn right to intercept |
| 210° | 150° left | Parallel: fly 180° for about 1 minute, then turn left back to the fix |
| 330° | 30° left | Direct |

Make the same hold a left-turn hold and the 090° arrival becomes a parallel entry, with a right turn back to the fix. The 270° arrival becomes a direct entry.

## The 5° either-entry band

Headings are never exact, and a heading right on a sector edge could be read either way. The FAA AIM says to pick the entry from your heading on arrival at the fix, and counts 5° either way as within good operating limits for that choice. The Transport Canada AIM says the same thing in other words: enter by the three sectors, with a zone of flexibility of five degrees on either side of the boundaries.

The tool marks the band. For the same right-turn hold, a heading of 110° is a direct entry on the boundary, and the tool adds that a teardrop is also acceptable. At 113°, it picks teardrop and says a direct entry is also acceptable. At 290° it picks direct and notes a parallel entry would be acceptable too.

## An example from the Canadian AIM

The Transport Canada AIM gives a missed approach that ends with a right-turn hold at the ZHZ beacon on an inbound track of 234°. Arriving on that same heading of 234°, you are flying straight down the holding course: the tool gives a direct entry, turning right to the outbound course of 054°.

## Common mistakes

- **Using the wrong course.** The sectors are measured from the inbound course, not from the radial or bearing named in the clearance. Holding east of a VOR on the 090 radial means an inbound course of 270°.
- **Deciding too early.** The entry depends on your heading at the fix. A turn on the way in, or wind, can change it.
- **Forgetting left turns.** A nonstandard hold mirrors every sector. Redo the picture.
- **Crossing the fix fast.** Slow to the holding speed before the fix. The [maximum holding airspeed tool](/aviation/ifr/hold-speed-limit/) gives the AIM limit for your altitude.
- **Ignoring the wind.** The entry legs drift like any other. The [holding wind correction tool](/aviation/ifr/hold-wind-timing/) works out the outbound heading and timing.

## Where the numbers come from

The sectors and entry procedures are in AIM paragraph 5-3-8, and the FAA *Instrument Flying Handbook* covers wind correction and timing in holds. The 5° band is in the same AIM paragraph and in Transport Canada's AIM, RAC 10.5. Every entry above comes from the [holding entry tool](/aviation/ifr/hold-entry/). This is a planning and education aid; ATC instructions and the published procedure govern.
