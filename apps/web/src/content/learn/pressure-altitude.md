---
title: What is pressure altitude?
description: Pressure altitude is what your altimeter reads set to 29.92 inHg. How to work it out from field elevation and the altimeter setting, and where the 1,000 ft rule drifts.
summary: The altitude on a standard altimeter setting, why 29.92 matters, and how QNH, QFE, and QNE fit together.
audience: Pilots
published: 2026-09-23
tools:
  - aviation.altimetry.pressure-altitude
  - aviation.altimetry.q-codes
  - aviation.altimetry.flight-level
  - aviation.altimetry.density-altitude
sources:
  - title: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
    issuer: Federal Aviation Administration
    edition: 2023
    locator: Chapter 8, Flight Instruments (types of altitude), and Chapter 11, Aircraft Performance (pressure altitude)
    url: https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/phak
  - title: Aviation Weather Handbook (FAA-H-8083-28B)
    issuer: Federal Aviation Administration
    edition: 2026
    locator: Section 8.4, Altimetry (altimeter setting and pressure altitude)
    url: https://www.faa.gov/sites/faa.gov/files/FAA-H-8083-28B.pdf
  - title: Aeronautical Information Manual
    issuer: Federal Aviation Administration
    edition: Current edition
    locator: Chapter 7, Section 2, Barometric Altimeter Errors and Setting Procedures (7-2-3)
    url: https://www.faa.gov/air_traffic/publications/atpubs/aim_html/chap7_section_2.html
  - title: 14 CFR 91.121, Altimeter settings
    issuer: Electronic Code of Federal Regulations
    edition: Current as of 2026-09-01
    locator: Paragraphs (a) and (b), the 18,000 ft rule and the lowest usable flight level table
    url: https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-91/subpart-B/subject-group-ECFRe4c59b5f5506932/section-91.121
---

Pressure altitude is the altitude your altimeter shows when you set its window to 29.92 inHg (1013.25 hPa). It is height measured in the standard atmosphere, not height above the sea. Performance charts, density altitude, and flight levels all start from it.

On a day when the air pressure at sea level is exactly standard, pressure altitude equals the altitude above sea level. Most days it is not. Low pressure puts pressure altitude above field elevation, and high pressure puts it below.

## Why it matters

Your airplane's takeoff and climb charts use pressure altitude, not field elevation. A low-pressure day makes a runway act higher than it is, and it adds straight into density altitude. Pressure altitude is also what you fly above 18,000 ft: there, every pilot sets 29.92 inHg, so all aircraft measure height from the same pressure level and stay separated from each other.

## The altimeter settings, in one place

Pilots and controllers talk about three settings. The names come from the old radio Q-codes.

| Name | What you set | What the altimeter reads on the ground |
|---|---|---|
| QNH, the altimeter setting | The local setting from ATIS, the tower, or a METAR | Field elevation |
| QFE | The station pressure at the field | Zero |
| QNE | 29.92 inHg (1013.25 hPa), the standard setting | The field's pressure altitude |

In the United States you fly on QNH below 18,000 ft and on 29.92 at or above it (14 CFR 91.121). QFE shows up mainly outside the United States. The [QNH, QFE, and QNE tool](/aviation/altimetry/q-codes/) converts between all three.

## Why 29.92?

The standard atmosphere, which ICAO defines, has a sea-level pressure of 1013.25 hPa. That is 29.92 inHg. Altimeters are built to that model: the scale turns pressure into feet as if every day were standard. The window lets you slide the scale so the needle matches today's pressure. Set 29.92 and you read the standard atmosphere with no adjustment, which is pressure altitude.

## How it is worked out

The exact method has two steps.

1. **The pressure altitude of the setting.** Ask what height in the standard atmosphere has the same pressure as the altimeter setting. A setting below 29.92 sits above sea level in the standard atmosphere. A setting above 29.92 sits below it.
2. **Add field elevation.** An altimeter set to QNH reads field elevation on the ground. Pressure altitude is field elevation plus the pressure altitude of the setting.

The [pressure altitude tool](/aviation/altimetry/pressure-altitude/) does both steps with the ICAO standard atmosphere. It also gives the station pressure, the barometer reading at the field.

## A worked example

A 5,000 ft airport with an altimeter setting of 29.80 inHg:

| Step | Result |
|---|---|
| Station pressure at the field | 24.79 inHg |
| Pressure altitude, exact | **5,112 ft** |
| Pressure altitude, 1,000 ft per inch | 5,120 ft |
| Rule of thumb error | 8 ft |

The setting is 0.12 inHg below standard, so the airplane is at a pressure altitude about 112 ft above the runway before it moves.

## The rule of thumb, and how far it drifts

The shortcut is 1,000 ft for each inch of mercury. The Aeronautical Information Manual uses the same figure for the cost of a wrong setting: an inch of error in the altimeter is 1,000 ft of altitude. In pressure altitude terms:

> pressure altitude ≈ field elevation + (29.92 − altimeter setting) × 1,000 ft

Near standard, the rule is very close. It drifts as the setting moves away from 29.92, because near sea level an inch of mercury is worth a bit over 900 ft, not 1,000. In the table below, one inch works out to 939 ft on the low side and 911 ft on the high side. Some results from the tool:

| Field elevation | Altimeter setting | Exact | 1,000 ft per inch | Error |
|---|---|---|---|---|
| 5,000 ft | 29.80 inHg | 5,112 ft | 5,120 ft | 8 ft |
| 0 ft | 30.42 inHg | -458 ft | -500 ft | 42 ft |
| 5,000 ft | 28.92 inHg | 5,939 ft | 6,000 ft | 61 ft |
| 5,000 ft | 30.92 inHg | 4,089 ft | 4,000 ft | 89 ft |

On a normal day the rule is well within what you can read off a chart. Far from standard, it can be off by close to 100 ft.

Even setting 29.92 at a sea-level field gives 1 ft, not 0 ft. The reason: 1013.25 hPa is 29.921 inHg, and the rounded 29.92 is a hair lower.

## Flight levels

Above 18,000 ft in the United States, altitudes become flight levels. A flight level is pressure altitude in hundreds of feet: FL185 is a pressure altitude of 18,500 ft. When the local setting is low, the low flight levels sit closer to the airplanes flying below 18,000 ft on QNH. So 14 CFR 91.121(b) raises the lowest usable flight level as the setting falls. At 29.92 or higher it is FL180. From 29.91 through 29.42 it is FL185, and so on, 500 ft for each half inch.

With the setting at 29.42 inHg, the [flight level tool](/aviation/altimetry/flight-level/) gives FL185 as the lowest usable flight level. It also says that FL185 is 18,033 ft on the local altimeter setting: just above the 18,000 ft that the traffic below may be flying.

## Common mistakes

- **Using field elevation for performance.** Charts ask for pressure altitude. On a low-pressure day, field elevation understates it.
- **Getting the sign backward.** Low setting, higher pressure altitude. "High to low, look out below" is the same idea in flight: fly from high pressure into low without resetting, and the airplane is lower than the altimeter says.
- **Mixing inches and hectopascals.** 1013.25 hPa and 29.92 inHg are both standard. Hearing 996 hPa and setting 29.96 inHg is a known error.
- **Treating pressure altitude as your height.** It is a pressure level. True altitude also depends on temperature: in cold air you are lower than the altimeter shows.

## Where the numbers come from

The standard atmosphere is defined in ICAO Doc 7488. The FAA's *Pilot's Handbook of Aeronautical Knowledge* and *Aviation Weather Handbook* define pressure altitude and explain the altimeter setting. The rules for when to use 29.92 and the lowest usable flight level are in 14 CFR 91.121. Start with the [pressure altitude tool](/aviation/altimetry/pressure-altitude/) and carry the result into the [density altitude tool](/aviation/altimetry/density-altitude/), or read [What is density altitude?](/learn/density-altitude/) for the next step.
