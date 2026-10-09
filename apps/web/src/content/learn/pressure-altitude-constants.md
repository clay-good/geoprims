---
title: Why two calculators disagree on pressure altitude
description: The ISA-derived constants 145,442.16 ft and 0.190263 against the NWS set 145,366.45 ft and 0.190284, where each comes from, and how far apart they land.
summary: The two constant sets behind pressure altitude, why they differ, and when the gap grows from a few feet to tens of feet.
audience: Pilots
published: 2026-10-09
tools:
  - aviation.altimetry.pressure-altitude
  - aviation.altimetry.q-codes
  - aviation.atmosphere.isa
sources:
  - title: Manual of the ICAO Standard Atmosphere (Doc 7488/3)
    issuer: International Civil Aviation Organization
    edition: 3rd edition, 1993
    locator: Section 2, defining constants and equations
    url: https://store.icao.int/en/manual-of-the-icao-standard-atmosphere-extended-to-80-kilometres-262500-feet-doc-7488
  - title: Pressure Altitude (WxCalc)
    issuer: National Weather Service, El Paso
    edition: Web calculator documentation
    locator: Pressure altitude from station pressure in millibars
    url: https://www.weather.gov/media/epz/wxcalc/pressureAltitude.pdf
  - title: Station Pressure (WxCalc)
    issuer: National Weather Service, El Paso
    edition: Web calculator documentation
    locator: Station pressure from altimeter setting and elevation
    url: https://www.weather.gov/media/epz/wxcalc/stationPressure.pdf
---

Two pressure-altitude formulas are in common use, and they give slightly different answers for the same barometer reading. Both have the same shape:

> pressure altitude = scale × (1 − (p ÷ 1013.25 hPa)^exponent)

where p is the station pressure. They differ only in the two numbers:

| Constant set | Scale | Exponent | Used by |
|---|---|---|---|
| ISA-derived | 145,442.16 ft | 0.190263 | This site, by default |
| National Weather Service | 145,366.45 ft | 0.190284 | The NWS WxCalc calculators on weather.gov |

If you check a pressure altitude here against a weather.gov calculator and the two differ by a few feet, this is why. Neither is a typo. They are rounded from slightly different starting values.

## Where each set comes from

The International Standard Atmosphere, defined by ICAO in Doc 7488, starts at 15 °C (288.15 K) and 1013.25 hPa at sea level. Temperature falls 6.5 °C per kilometer up to the tropopause. In that layer, the height at a pressure follows directly from those values.

**The scale** is the sea-level temperature divided by the lapse rate: the height at which the temperature would reach absolute zero if the lapse went on forever. With 288.15 K, that is 44,330.8 m, or 145,442.16 ft. The NWS figure, 145,366.45 ft, is what you get with 288 K instead: 288 ÷ 0.0065 = 44,307.7 m, or 145,366.45 ft. The NWS set rounds the sea-level temperature to a whole kelvin.

**The exponent** is the gas constant for air times the lapse rate, divided by standard gravity. With ICAO's values (287.05287 J/(kg·K), 0.0065 K/m, and 9.80665 m/s²) it is 0.190263. The NWS exponent, 0.190284, is a little larger. It is written down as given, without a derivation.

## How far apart they land

Given the same station pressure, the constants alone move the answer by only a few feet:

| Station pressure | ISA-derived | NWS | Difference |
|---|---|---|---|
| 1050 hPa | −989 ft | −989 ft | under 1 ft |
| 900 hPa | 3,243 ft | 3,242 ft | 1 ft |
| 800 hPa | 6,394 ft | 6,391 ft | 3 ft |
| 700 hPa | 9,882 ft | 9,878 ft | 4 ft |
| 500 hPa | 18,289 ft | 18,281 ft | 8 ft |

That is far below anything a pilot can read on an altimeter, and it is zero at 1013.25 hPa, where both formulas give 0 ft.

## The bigger gap: getting station pressure

In practice you rarely start from station pressure. You start from the field elevation and the altimeter setting, and the station pressure has to be worked out first. That step is where the two calculators really part ways.

An altimeter set to the local setting reads the field elevation on the ground. In the standard atmosphere, that puts the field at the height the setting itself corresponds to, plus the elevation. This site finds the station pressure that way, which makes the pressure altitude simply the elevation plus the setting's own pressure altitude.

The NWS station-pressure formula takes a shortcut instead. It multiplies the altimeter setting by the standard pressure ratio for the field's elevation, ((288 − 0.0065 × elevation in m) ÷ 288)^5.2561. That is exact only at sea level or when the setting is 29.92 inHg. Away from those, the two station pressures drift apart, and the error grows with elevation and with how far the setting is from standard.

| Field | Setting | This site (ISA) | NWS calculator | 1,000 ft rule |
|---|---|---|---|---|
| 5,000 ft | 29.80 inHg | 5,112 ft | 5,109 ft | 5,120 ft |
| 5,434 ft | 30.12 inHg | 5,251 ft | 5,258 ft | 5,234 ft |
| 9,000 ft | 28.90 inHg | 9,958 ft | 9,900 ft | 10,020 ft |
| 12,000 ft | 29.50 inHg | 12,392 ft | 12,361 ft | 12,420 ft |

At a high field with a low setting, the two calculators can be 50 ft or more apart. That is still small next to the rule of thumb, which is off by 62 ft in the 9,000 ft case, but it is large enough to be noticed when you compare answers.

## Which one to use

For performance charts, density altitude, and true airspeed, either set is fine: the charts themselves are read to the nearest few hundred feet. This site uses the ISA-derived constants and the altimeter-setting relation because they follow from the standard atmosphere's own definition, and they agree with the ISA tables at every altitude.

If you need to match a weather.gov answer exactly, for instance when checking a forecast product or teaching from the NWS worksheets, choose **nws** under "Constant set" in the [pressure altitude tool](/aviation/altimetry/pressure-altitude/). It then uses the NWS station-pressure and pressure-altitude formulas as published, shortcut included, and "How we got this" shows both steps with your numbers.

To go the other way, from a barometer reading at the field to the settings a tower would give, use the [QNH, QFE, and QNE tool](/aviation/altimetry/q-codes/). To see the standard atmosphere these formulas come from, with its layers and the tropopause at 36,089 ft, use the [standard atmosphere tool](/aviation/atmosphere/isa/).
