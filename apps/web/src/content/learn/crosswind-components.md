---
title: How to calculate crosswind
description: Split the wind into headwind and crosswind for your runway, with gusts, the clock-face rule and how far it drifts, and what maximum demonstrated crosswind means.
summary: Headwind and crosswind from the runway and the reported wind, the clock-face shortcut, gusts, and your airplane's demonstrated crosswind.
audience: Pilots
published: 2026-09-23
tools:
  - aviation.wind.runway-components
  - aviation.wind.best-runway
  - aviation.weather.metar-decode
sources:
  - title: Airplane Flying Handbook (FAA-H-8083-3C)
    issuer: Federal Aviation Administration
    edition: 2021
    locator: Chapter 9, Approaches and Landings (crosswind approach and landing, maximum crosswind velocities, common errors)
    url: https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/airplane_handbook
  - title: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
    issuer: Federal Aviation Administration
    edition: 2023
    locator: Chapter 11, Aircraft Performance (crosswind and headwind component chart, page 11-25)
    url: https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/phak
  - title: Aviation Weather Handbook (FAA-H-8083-28B)
    issuer: Federal Aviation Administration
    edition: 2026
    locator: Section 3.4.2.2, Products with Wind Information (magnetic in ATIS and ASOS/AWOS voice, true elsewhere)
    url: https://www.faa.gov/sites/faa.gov/files/FAA-H-8083-28B.pdf
  - title: Aeronautical Information Manual
    issuer: Federal Aviation Administration
    edition: Current edition
    locator: Chapter 2, Section 3, Airport Marking Aids and Signs (runway designators)
    url: https://www.faa.gov/air_traffic/publications/atpubs/aim_html/chap2_section_3.html
---

To calculate crosswind, find the angle between the wind and the runway, then multiply the wind speed by the sine of that angle. The headwind is the wind speed times the cosine of the same angle. A wind 30° off the runway gives a crosswind of half the wind speed; a wind straight across gives all of it.

Every wind that is not straight down the runway splits into two parts. The part along the runway is a headwind or tailwind. The part across it is the crosswind, which pushes the airplane sideways and must be held off with the controls on takeoff and landing.

## Why it matters

The headwind shortens your takeoff and landing rolls, and a tailwind lengthens them. The crosswind decides whether you can keep the airplane lined up with the centerline at all. The *Airplane Flying Handbook* lists landing in a crosswind beyond the airplane's maximum demonstrated crosswind component as the first common error in crosswind landings. Working out the components before you pick a runway, or before you decide to go, takes a minute.

## How it is worked out

1. **Get the runway heading.** A runway number is its magnetic heading to the nearest 10°, divided by 10. Runway 27 is about 270° magnetic. The published heading can differ from the number by up to 5°.
2. **Get the wind in the same reference.** Winds from ATIS, the tower, and ASOS or AWOS voice broadcasts are magnetic. Winds in a METAR or TAF are true. Where magnetic variation is large, mixing the two can move the angle by 10° or more.
3. **Find the angle** between the wind direction and the runway heading.
4. **Split the wind.** Crosswind = wind × sin(angle). Headwind = wind × cos(angle). If the angle is more than 90°, the "headwind" comes out negative: it is a tailwind.
5. **Do the same for the gust.** Gusts use the same angle and the gust speed.

The [runway wind components tool](/aviation/wind/runway-components/) does all five steps. It reads a METAR wind group like 30015G25KT directly, handles variable winds, and converts between true and magnetic if you give it the variation.

## A worked example

Runway 27 with the wind 300° at 15 kt gusting 25 kt, and a personal crosswind limit of 15 kt:

| Item | Result |
|---|---|
| Runway heading used | 270° |
| Angle between wind and runway | 30° |
| Crosswind | **7.5 kt from the right** |
| Headwind | 13 kt |
| Crosswind in the gusts | 12.5 kt |
| Headwind in the gusts | 21.7 kt |
| Against your limit | Within your 15 kt crosswind limit |

The steady crosswind is half the wind, as the 30° angle promises. The gusts bring it to 12.5 kt, still inside the 15 kt limit but with less margin. The tool also warns that runway 27 was taken as 270°; enter the published heading for exact components.

## The clock-face rule, and how far it drifts

The clock-face rule treats the angle between wind and runway as minutes on a clock. At 15 minutes past, a quarter of the hour has gone, so a 15° angle gives a quarter of the wind as crosswind. At 30° it is half, at 45° three quarters, and at 60° or more the whole wind.

For a 20 kt wind on runway 27, the tool gives:

| Angle off the runway | Exact crosswind | Clock-face rule | Exact headwind |
|---|---|---|---|
| 15° | 5.2 kt | 5 kt | 19.3 kt |
| 30° | 10 kt | 10 kt | 17.3 kt |
| 45° | 14.1 kt | 15 kt | 14.1 kt |
| 60° | 17.3 kt | 20 kt | 10 kt |
| 90° | 20 kt | 20 kt | 0 kt |

The rule is exact at 30° and 90°. It reads high at 45° and 60°, by up to 2.7 kt here, which errs on the cautious side. At 15° it reads 0.2 kt low, too little to matter. The rule gives you nothing about the headwind; note that at 60° off, half the wind is still on the nose.

## Choosing a runway

At an airport with several runways, work the components for each one. The [runway ranking tool](/aviation/wind/best-runway/) does that in one step. For runways 09/27 and 18/36 with the wind 200° at 12 kt, it ranks runway 18 first, with 11.3 kt of headwind and 4.1 kt of crosswind. Runway 27 would give 11.3 kt of crosswind from the left. ATC, noise rules, and runway length may still point you elsewhere.

## Maximum demonstrated crosswind

The *Airplane Flying Handbook* describes the test: before type certification, an airplane is flight tested for control in 90° crosswinds up to a speed equal to 0.2 V<sub>S0</sub>, two-tenths of its power-off stalling speed in the landing configuration. The *Pilot's Handbook of Aeronautical Knowledge* gives the example of a 45 kt stalling speed, which means a 9 kt crosswind. The figure the manufacturer demonstrated is published in the POH or AFM, and on a placard in airplanes certificated after May 3, 1962.

It is what was shown in flight testing, not a promise about your skill on the day. Your own limit may be lower; set it from your recent practice and put it in the tool's limit box.

## Common mistakes

- **Mixing true and magnetic.** A METAR wind is true. The runway and the ATIS wind are magnetic. Convert before you compare.
- **Ignoring the gusts.** The gust crosswind is what you meet in the flare.
- **Forgetting the tailwind.** A wind more than 90° off the runway adds to your ground roll.
- **Trusting the runway number to the degree.** It can be 5° off the real heading. On a strong wind nearly along the runway, that changes the crosswind noticeably.

## Where the numbers come from

The method is the vector split shown in the FAA's crosswind component charts in the *Pilot's Handbook of Aeronautical Knowledge*, done exactly instead of by eye. The *Airplane Flying Handbook* covers crosswind technique and the certification test. To get the wind straight from a report, start with the [METAR decoder](/aviation/weather/metar-decode/), then carry it into the [runway wind components tool](/aviation/wind/runway-components/). This is a planning and education aid; your POH or AFM and the current reported wind govern.
