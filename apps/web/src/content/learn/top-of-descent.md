---
title: When to start descending
description: Where to start down from cruise, the 3° path and its 318 ft per NM, the 3-to-1 and 5 × groundspeed rules, and the visual descent point on an approach.
summary: Top of descent from cruise altitude and groundspeed, the rules of thumb and how far they drift, and the visual descent point on a non-precision approach.
audience: Pilots
published: 2026-09-23
tools:
  - aviation.performance.top-of-descent
  - aviation.performance.vdp
sources:
  - title: Instrument Procedures Handbook (FAA-H-8083-16B)
    issuer: Federal Aviation Administration
    edition: FAA-H-8083-16B
    locator: Chapter 2, En Route Operations (top of descent), and Chapter 4, Approaches (descent rates, the 300 ft per NM check, VDP)
    url: https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/instrument_procedures_handbook
  - title: Aeronautical Information Manual
    issuer: Federal Aviation Administration
    edition: Current edition
    locator: Chapter 5, Section 4, paragraph 5-4-5, Instrument Approach Procedure Charts (visual descent point)
    url: https://www.faa.gov/air_traffic/publications/atpubs/aim_html/chap5_section_4.html
  - title: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
    issuer: Federal Aviation Administration
    edition: 2023
    locator: Chapter 16, Navigation (flight planning, time and distance)
    url: https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/phak
---

Start descending when the distance to go equals the altitude to lose divided by the slope you want to fly. On the common 3° path, that is about 318 ft of descent for every nautical mile, so a rough answer is 3 NM for every 1,000 ft to lose. The rate to descend at is about 5 times your groundspeed in feet per minute.

The point where you leave cruise altitude is the top of descent, often shortened to TOD. Flight management systems compute it for you. Without one, the arithmetic is short enough to do in your head, once you know where the shortcuts come from.

## Why it matters

Start down too late and you arrive high: a steep, fast descent, a rushed approach, or a go-around. Start too early and you spend fuel low and slow, maybe in rougher air or under a cloud deck you meant to stay above. A planned descent lets you keep the power up, the passengers comfortable, and the engine warm.

## How it is worked out

A steady descent is a straight line from cruise altitude to the target altitude.

1. **Altitude to lose.** Cruise altitude minus the target: pattern altitude, an approach fix altitude, or a crossing restriction.
2. **Distance.** Altitude to lose ÷ tan(descent angle). A 3° path drops 318 ft per nautical mile.
3. **Vertical speed.** Groundspeed × tan(descent angle), in feet per minute. The same angle needs a faster descent rate at a higher groundspeed.
4. **Time.** Distance ÷ groundspeed.

If you would rather fly a set vertical speed, turn it around: the angle follows from the vertical speed and groundspeed, and the distance from the angle. The [top of descent tool](/aviation/performance/top-of-descent/) works either way.

## A worked example

From FL350 down to 3,000 ft at 420 kt groundspeed, on a 3° path:

| Item | Result |
|---|---|
| Altitude to lose | 32,000 ft |
| Gradient | 318 ft/NM |
| Distance to start down | **100.5 NM** |
| Vertical speed | 2,229 ft/min |
| Time | 14.4 min |

A light airplane at 8,500 ft that wants to reach 2,000 ft on the same 3° path has 20.4 NM to go. At 140 kt that is 743 ft/min for 8.7 min. At 90 kt the distance does not change, but the rate drops to 478 ft/min and the descent takes 13.6 min.

## Rules of thumb, and how far they drift

**The 3-to-1 rule.** Allow 3 NM for each 1,000 ft to lose. It treats the path as 333 ft per NM instead of 318, so it starts you down a little late.

**Five times groundspeed.** For a 3° path, descend at 5 × groundspeed in ft/min. The exact figure is about 5.3 × groundspeed, so the rule descends a little slowly.

| Case | Exact distance | 3-to-1 rule | Exact vertical speed | 5 × groundspeed |
|---|---|---|---|---|
| FL350 to 3,000 ft at 420 kt | 100.5 NM | 96 NM | 2,229 ft/min | 2,100 ft/min |
| 8,500 to 2,000 ft at 140 kt | 20.4 NM | 19.5 NM | 743 ft/min | 700 ft/min |
| 8,500 to 2,000 ft at 90 kt | 20.4 NM | 19.5 NM | 478 ft/min | 450 ft/min |

Both rules err the same way: start a little late, descend a little slowly. The two errors add, so on a long descent you arrive high. The 3-to-1 rule was 4.5 NM short for the airliner. Adding a few miles for slowing down and for a tailwind, and checking your height against the distance as you go, closes the gap.

If you prefer a gentle 500 ft/min, the path is shallower. From 8,500 to 2,000 ft at 140 kt, the tool gives a 2.02° path of 214 ft/NM, starting 30.3 NM out and taking 13 min.

## The visual descent point

The same geometry applies at the bottom of a non-precision approach. The *Aeronautical Information Manual* defines the visual descent point (VDP) as a point on the final approach course from which a stabilized visual descent from the minimum descent altitude (MDA) to the runway can begin. Do not descend below the MDA before reaching it.

When the chart shows no VDP, the *Instrument Procedures Handbook* suggests working out a normal descent point yourself, using 300 ft per NM. Divide the height above touchdown by 300 to get the distance from the threshold.

For an MDA 400 ft above the touchdown zone, at 90 kt, the [visual descent point tool](/aviation/performance/vdp/) gives:

| Item | Result |
|---|---|
| VDP, exact 3° path to the threshold | **1.26 NM** |
| VDP, height ÷ 300 | 1.33 NM |
| Difference | 0.08 NM |
| Descent rate from the VDP | 478 ft/min |
| Time to the threshold | 50 s |

The handbook's own example is the same case, an MDA 400 ft above the touchdown zone, and it puts the airplane about 1.3 NM from the threshold. Plan to cross the threshold 50 ft up instead of at the pavement, and the tool moves the VDP in to 1.1 NM. A VDP printed on the chart always governs over your own number.

## Common mistakes

- **Using airspeed instead of groundspeed.** A tailwind raises the descent rate a 3° path needs. At a set descent rate, it moves the top of descent farther out.
- **Forgetting to slow down.** The distance assumes a steady groundspeed. Slowing for the pattern or an approach speed limit needs extra miles.
- **Descending to the airport elevation.** Plan to the pattern altitude or the approach fix altitude, not the field.
- **Pushing the rate too high near the ground.** The *Instrument Procedures Handbook* calls a descent rate above about 1,000 ft/min unacceptable in the final stages of an approach, below 1,000 ft above the ground.

## Where the numbers come from

The method and the 300 ft per NM check come from the FAA's *Instrument Procedures Handbook*. The VDP definition is in paragraph 5-4-5 of the *Aeronautical Information Manual*. Every value above comes from the [top of descent tool](/aviation/performance/top-of-descent/) and the [visual descent point tool](/aviation/performance/vdp/). This is a planning and education aid; ATC instructions and the published procedure govern.
