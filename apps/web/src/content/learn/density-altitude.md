---
title: What is density altitude?
description: Density altitude is the altitude your airplane performs at. How to work it out from field elevation, altimeter setting, and temperature, and where the rules of thumb drift.
summary: The altitude the airplane feels, why hot and high days lengthen the takeoff roll, and how to work it out exactly.
audience: Pilots
published: 2026-09-23
tools:
  - aviation.altimetry.density-altitude
  - aviation.altimetry.pressure-altitude
  - aviation.atmosphere.isa
  - aviation.weather.metar-decode
sources:
  - title: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
    issuer: Federal Aviation Administration
    edition: 2023
    locator: Chapter 11, Aircraft Performance (pressure altitude and density altitude)
    url: https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/phak
  - title: Aviation Weather Handbook (FAA-H-8083-28B)
    issuer: Federal Aviation Administration
    edition: 2026
    locator: Section 8.4, Altimetry
    url: https://www.faa.gov/sites/faa.gov/files/FAA-H-8083-28B.pdf
  - title: Manual of the ICAO Standard Atmosphere (Doc 7488/3)
    issuer: International Civil Aviation Organization
    edition: 3rd edition, 1993
    locator: Section 2, defining constants and equations
    url: https://store.icao.int/en/manual-of-the-icao-standard-atmosphere-extended-to-80-kilometres-262500-feet-doc-7488
---

Density altitude is the altitude in the standard atmosphere where the air has the same density as the air around your airplane right now. Wings, propellers, and engines respond to air density, not to the number on the altimeter, so density altitude is the altitude the airplane actually performs at.

On a cool day at a sea-level airport, density altitude is close to field elevation. On a hot afternoon at a mountain airport, it can be thousands of feet higher. The airplane then takes off, climbs, and lands as if the runway were that much higher, with a longer takeoff roll, a lower rate of climb, and a faster true airspeed for the same indicated airspeed on approach.

## Why it matters

Your pilot's operating handbook (POH) gives takeoff distance and climb rate by pressure altitude and temperature. Density altitude is the single number that combines both, which is why it is the quickest way to sense how a day will fly. An airplane that is legal and within weight can still fail to clear an obstacle off a short, high runway on a hot afternoon, so it is worth checking before every summer departure from a high field.

Three things raise density altitude:

- **Height.** Air is thinner at a higher field.
- **Heat.** Warm air is less dense than cold air at the same pressure.
- **Humidity.** Water vapor is lighter than dry air, so moist air is slightly less dense.

Pilots remember them as "high, hot, and humid."

## How it is worked out

The exact method has three steps.

1. **Pressure altitude.** An altimeter set to the local altimeter setting (QNH) reads field elevation on the ground. Pressure altitude is what it would read with the setting at 29.92 inHg (1013.25 hPa). It equals field elevation plus the pressure altitude of the altimeter setting itself. The [pressure altitude tool](/aviation/altimetry/pressure-altitude/) does this step.
2. **Air density.** From the station pressure and the outside air temperature, the ideal gas law gives the air's density. With a dew point, the tool uses the virtual temperature, which accounts for water vapor.
3. **Density altitude.** Find the altitude in the ICAO standard atmosphere that has that density. The standard atmosphere starts at 15 °C and 1013.25 hPa at sea level and cools 1.98 °C per 1,000 ft. The [standard atmosphere tool](/aviation/atmosphere/isa/) gives its values at any altitude.

## A worked example

A 5,000 ft airport, an altimeter setting of 29.80 inHg, and 30 °C on the ramp:

| Step | Result |
|---|---|
| Pressure altitude | 5,112 ft |
| Standard temperature at that altitude | 4.9 °C |
| How much warmer than standard | 25.1 °C |
| Density altitude, dry air | **7,937 ft** |

The airplane performs as if it were at almost 8,000 ft, about 2,900 ft above the runway. Add a dew point of 20 °C and density altitude rises to 8,277 ft: humidity alone adds another 340 ft.

## The rule of thumb, and how far it drifts

The common shortcut adds about 120 ft to pressure altitude for every degree Celsius above standard temperature. Some references use 118.8 ft. For the example above:

| Method | Density altitude | Error |
|---|---|---|
| Exact, dry air | 7,937 ft | none |
| 118.8 ft per °C | 8,098 ft | 161 ft high |
| 120 ft per °C | 8,128 ft | 191 ft high |

On a dry day the rule of thumb errs on the safe side, reading a little high. On a humid day it can read low: with the 20 °C dew point it is 150 ft under the exact figure, because the rule knows nothing about moisture.

## Common mistakes

- **Using field elevation instead of pressure altitude.** A low altimeter setting raises pressure altitude, and density altitude with it.
- **Using the forecast high from the morning briefing.** Take the temperature you will actually depart in.
- **Ignoring humidity.** Most E6B computers and charts assume dry air.
- **Treating density altitude as performance.** It tells you how the air behaves. Your POH turns that into takeoff distance and climb rate for your airplane and weight.

## Where the numbers come from

The standard atmosphere is defined in ICAO Doc 7488, and the FAA's *Pilot's Handbook of Aeronautical Knowledge* and *Aviation Weather Handbook* describe how pilots use it. The [density altitude tool](/aviation/altimetry/density-altitude/) shows each step with your numbers under "How we got this." To read the altimeter setting and temperature straight from a weather report, start with the [METAR decoder](/aviation/weather/metar-decode/).
