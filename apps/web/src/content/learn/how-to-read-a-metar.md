---
title: How to read a METAR
description: A METAR is an airport weather report in code. Read it group by group, find the ceiling, and work out the flight category, with a decoded example.
summary: The groups of a METAR in the order they come, what each one means, and how the ceiling and visibility set the flight category.
audience: Pilots
published: 2026-09-23
tools:
  - aviation.weather.metar-decode
  - aviation.weather.taf-decode
  - aviation.altimetry.density-altitude
  - aviation.altimetry.pressure-altitude
sources:
  - title: Key to Aerodrome Forecast (TAF) and Aviation Routine Weather Report (METAR)
    issuer: Federal Aviation Administration and National Weather Service
    edition: Key card, as posted by NWS New York
    locator: Front (each METAR group) and back (weather codes, ceiling)
    url: https://www.weather.gov/media/okx/Aviation/TAF_Card.pdf
  - title: Aviation Weather Handbook, Chapter 3, Overview of Aviation Weather Information
    issuer: Federal Aviation Administration (posted by the FAA Safety Team)
    edition: FAA-H-8083-28 series, 2024 posting
    locator: Section 3.4.2.14, Products with Surface-Based IFR Information (flight categories)
    url: https://www.faasafety.gov/files/events/SO/SO15/2024/SO15129448/FAA-H-8083-28Chpt3.pdf
---

A METAR is a routine weather observation from an airport, written in a short code. It always runs in the same order: station, time, wind, visibility, weather, clouds, temperature and dew point, and altimeter setting, then remarks. Once you know the order, each group has one job, and the code reads like a sentence.

A SPECI is a special report in the same format. The station sends one between the hourly reports when the weather changes enough to matter.

## Why it matters

The METAR is what the weather is doing now, measured at the field. It tells you the wind for your runway, whether the clouds and visibility allow the flight you have planned, and the temperature and altimeter setting you need for [density altitude](/learn/density-altitude/). It is also the check on the forecast: if the METAR is worse than the [TAF](/learn/how-to-read-a-taf/) said it would be, the forecast is already wrong.

## The groups, in order

Here is a report from Denver on a gusty afternoon, decoded by the [METAR decoder](/aviation/weather/metar-decode/):

`KDEN 181753Z 30015G25KT 10SM FEW080 SCT200 30/08 A2980 RMK AO2 SLP052 T03000083`

| Group | Meaning |
|---|---|
| KDEN | Denver International, by its four-letter ICAO code |
| 181753Z | Observed on the 18th at 1753Z (UTC) |
| 30015G25KT | Wind 300° true at 15 kt, gusting 25 kt |
| 10SM | Visibility 10 statute miles |
| FEW080 | Few clouds at 8,000 ft |
| SCT200 | Scattered clouds at 20,000 ft |
| 30/08 | Temperature 30 °C, dew point 8 °C |
| A2980 | Altimeter setting 29.80 inHg |
| RMK | Remarks follow |
| AO2 | Automated station that can tell rain from snow |
| SLP052 | Sea-level pressure 1,005.2 hPa |
| T03000083 | Temperature 30.0 °C, dew point 8.3 °C, to a tenth |

The decoder reads this as VFR, with the wind 300° true at 15 kt, gusting 25 kt.

A few groups need more than a glance.

- **Time.** Two digits for the day of the month, then the time in UTC. There is no month or year.
- **Wind.** The first three digits are the direction the wind blows from, in degrees true, to the nearest 10°. Then the speed in knots, and G with the gust if there is one. 00000KT is calm. VRB means variable. A group like 180V260 after the wind means the direction swings across that range.
- **Visibility.** Statute miles and fractions in the US, like 3/4SM. Some reports add runway visual range, like R28L/2600FT.
- **Weather.** A minus sign means light, no sign moderate, and a plus sign heavy. Two-letter codes follow: RA rain, SN snow, TS thunderstorm, SH showers, BR mist, FG fog. So -SHRA is light rain showers and +TSRA is a thunderstorm with heavy rain. VC means in the vicinity, 5 to 10 miles away in the US.
- **Clouds.** FEW is up to 2/8 of the sky, SCT (scattered) 3/8 to 4/8, BKN (broken) 5/8 to 7/8, and OVC (overcast) all of it. The three digits are the base in hundreds of feet above the ground, so BKN025 is broken at 2,500 ft. VV002 is a sky hidden by fog or smoke, with a vertical visibility of 200 ft. CB marks a cumulonimbus. CLR, on automated reports only, means no clouds below 12,000 ft.
- **Temperature.** Degrees Celsius. An M in front means minus, so M06 is −6 °C.
- **Altimeter.** A and four digits is inches of mercury: A2980 is 29.80 inHg. Outside the US, Q and four digits is hectopascals.

## The ceiling and the flight category

The ceiling is the lowest layer reported as broken or overcast, or the vertical visibility. FEW and SCT layers are clouds, but they are not a ceiling, however low they are.

Weather websites color METARs by flight category. The FAA *Aviation Weather Handbook* gives the usual definitions:

| Category | Ceiling | Visibility |
|---|---|---|
| LIFR (low IFR) | below 500 ft | or below 1 SM |
| IFR | 500 ft to below 1,000 ft | or 1 SM to below 3 SM |
| MVFR (marginal VFR) | 1,000 ft to 3,000 ft | or 3 SM to 5 SM |
| VFR | above 3,000 ft | and above 5 SM |

The worse of the two decides. Four reports through the decoder:

| Report (main groups) | Ceiling | Visibility | Category |
|---|---|---|---|
| 10SM FEW080 SCT200 | none | 10 SM | VFR |
| 5SM HZ BKN025 OVC040 | 2,500 ft | 5 SM | MVFR |
| 2SM BR OVC008 | 800 ft | 2 SM | IFR |
| 1/4SM FG VV002 | 200 ft | 1/4 SM | LIFR |

The second report has 5 SM of visibility, which alone would be VFR, but the broken layer at 2,500 ft makes it MVFR. The handbook notes that these categories are for a quick picture of the weather. They are not the VFR weather minimums in 14 CFR 91.155, which depend on the airspace and your altitude.

## Remarks worth knowing

US remarks carry detail the body leaves out. AO2 says the station is automated and can sense the type of precipitation. AO1 means it cannot. SLP is the sea-level pressure in hectopascals with the leading 10 or 9 left off: SLP052 is 1,005.2 hPa. The T group gives the temperature and dew point to a tenth of a degree, with a 1 in front of either one meaning below zero. A dollar sign at the end means the station needs maintenance, so read its numbers with care.

## Common mistakes

- **Reading the time as local.** The Z means UTC. At Denver in summer, 1753Z is 11:53 am.
- **Taking a low FEW or SCT layer as the ceiling.** Only BKN, OVC, and VV count.
- **Reading cloud heights as altitudes.** They are heights above the airport, not above sea level.
- **Mixing up true and magnetic wind.** METAR and TAF winds are in degrees true. The tower and ATIS give the wind in degrees magnetic, to match the runway numbers. At Denver International the two differ by about 7°. See [true vs. magnetic north](/learn/true-vs-magnetic-north/).
- **Confusing mist and fog.** BR is mist, with visibility of 5/8 SM or more. FG is fog, with less.
- **Flying on an old report.** Check the time group. A METAR from two hours ago is history.

## Where the numbers come from

The group meanings follow the FAA and National Weather Service key to METAR and TAF codes, and the flight categories follow the FAA *Aviation Weather Handbook*. The [METAR decoder](/aviation/weather/metar-decode/) explains every group of any report you paste and lists any it cannot read, rather than dropping them. To read the forecast that goes with it, use the [TAF decoder](/aviation/weather/taf-decode/). To turn the temperature and altimeter setting into performance numbers, go on to [pressure altitude](/aviation/altimetry/pressure-altitude/) and [density altitude](/aviation/altimetry/density-altitude/). A decoded report is a study aid: get a current official briefing before flight.
