---
title: How to read a winds aloft forecast
description: How to decode the FB winds and temperatures aloft, including 9900 light and variable, the +50 code for winds of 100 kt or more, and the hidden minus sign.
summary: The coded FB winds aloft forecast group by group, the three rules that trip people up, and how to find the wind between two levels.
audience: Pilots
published: 2026-09-23
tools:
  - aviation.weather.fb-winds-decode
  - aviation.wind.aloft-interpolate
  - geodesy.magnetic.true-to-magnetic
sources:
  - title: Aviation Weather Handbook, Chapter 27, Forecasts
    issuer: Federal Aviation Administration (posted by the FAA Safety Team)
    edition: FAA-H-8083-28 series, 2024 posting
    locator: Section 27.2, Winds and Temperatures Aloft (issuance, text format, coding example, table 27-1, table 27-2)
    url: https://www.faasafety.gov/files/events/SO/SO15/2024/SO15129447/FAA-H-8083-28Chpt27.pdf
  - title: Key to Aerodrome Forecast (TAF) and Aviation Routine Weather Report (METAR)
    issuer: Federal Aviation Administration and National Weather Service
    edition: Key card, as posted by NWS New York
    locator: Front (wind direction in degrees true)
    url: https://www.weather.gov/media/okx/Aviation/TAF_Card.pdf
---

The winds and temperatures aloft forecast, called the FB, gives the forecast wind and temperature at set altitudes over a list of places. Each altitude gets one short group of digits, like 2714+05: wind from 270° true at 14 kt, temperature +5 °C. Three rules cover almost everything else: 9900 means light and variable, a direction code over 50 means the wind is 100 kt or more, and above 24,000 ft every temperature is negative even without a minus sign.

## Why it matters

The wind at your cruising altitude sets your groundspeed, your heading, and your fuel burn. The temperature aloft tells you where the freezing level is and feeds the true airspeed and density altitude at cruise. The FB is a long-standing, simple place to find both, and the National Weather Service still issues it.

The FAA *Aviation Weather Handbook* is frank about its limits. The FB is computer-made, updated only four times a day, and gives one value for each of three broad time periods at points about 100 to 150 miles apart. Flight planning software now pulls model winds that update every hour on a much finer grid. The FB is still the quickest check, and the code is worth knowing.

## How it is laid out

A product starts with a header, then one line per station:

- `DATA BASED ON 010000Z`
- `VALID 010600Z FOR USE 0500-0900Z. TEMPS NEG ABV 24000`
- `FT 3000 6000 9000 12000 18000 24000 30000 34000 39000`
- `MKC 9900 1709+06 2018+00 2130-06 2242-18 2361-30 247242 258848 750252`

Line by line:

- **DATA BASED ON** is the model run the forecast came from.
- **VALID** is the time the forecast is for, and **FOR USE** is the window to use it in.
- **FT** lists the altitudes, one column each.
- Each line after that is a station, like MKC for Kansas City, with one group per altitude.

A group has four digits for the wind, DDff, and at most levels a signed two-digit temperature after it. DD is the direction in tens of degrees true, the direction the wind blows from. ff is the speed in knots.

Some columns are blank on purpose. There is no wind forecast within 1,500 ft of a station's elevation and no temperature within 2,500 ft of it, and the 3,000 ft level carries no temperature at all. That is why the 3,000 ft group above is only four digits.

## The three rules

**9900 is light and variable.** It means a wind of less than 5 kt with no set direction. It is not a wind from 360°.

**A direction over 50 means 100 kt or more.** Two digits cannot hold a speed of 100 kt, so the code adds 50 to the direction and takes 100 off the speed. To decode, do the reverse: subtract 50 from the direction and add 100 to the speed. A group of 7799 means 270° at 199 kt or more, the most the code can say.

**Above 24,000 ft the minus sign is left out.** Temperatures up there are always below zero, so the header says TEMPS NEG ABV 24000 and the groups drop the sign. 247242 at 30,000 ft is −42 °C.

## A worked example

The group 731960 at 34,000 ft, decoded by the [winds aloft decoder](/aviation/weather/fb-winds-decode/):

| Step | Result |
|---|---|
| Direction code 73 is over 50, so subtract 50 | 23, or 230° |
| Speed code 19, plus 100 | 119 kt |
| Temperature 60, above 24,000 ft, so negative | −60 °C |
| Decoded | **230° true at 119 kt, −60 °C** |

The whole Kansas City line from the handbook's example, through the same decoder:

| Altitude | Group | Decoded |
|---|---|---|
| 3,000 ft | 9900 | Light and variable (less than 5 kt) |
| 6,000 ft | 1709+06 | 170° true at 9 kt, 6 °C |
| 9,000 ft | 2018+00 | 200° true at 18 kt, 0 °C |
| 12,000 ft | 2130-06 | 210° true at 30 kt, −6 °C |
| 18,000 ft | 2242-18 | 220° true at 42 kt, −18 °C |
| 24,000 ft | 2361-30 | 230° true at 61 kt, −30 °C |
| 30,000 ft | 247242 | 240° true at 72 kt, −42 °C |
| 34,000 ft | 258848 | 250° true at 88 kt, −48 °C |
| 39,000 ft | 750252 | 250° true at 102 kt, −52 °C |

The handbook's printed message shows that last group as 550252, which would decode to 050°. Its own table gives 250° at 102 kt, which only 750252 produces, so 750252 is the group used here.

## Between two levels

You rarely cruise at exactly 6,000 or 9,000 ft. The usual approach is to split the difference. With 270° at 20 kt, 3 °C at 6,000 ft and 300° at 30 kt, −3 °C at 9,000 ft, the [winds aloft interpolation tool](/aviation/wind/aloft-interpolate/) gives, for 7,500 ft:

| Method | Wind at 7,500 ft |
|---|---|
| Average the directions and speeds | 285° at 25 kt |
| Average the wind as a vector | **288° at 24.2 kt, 0 °C** |

The vector method treats each wind as an arrow, so a wind that turns with height turns smoothly and loses a little speed as it does. Here the difference is small, 3° and under 1 kt. It grows when the two levels disagree more. The interpolation tool is still marked experimental on this site.

## Common mistakes

- **Reading 9900 as calm from the north.** It is light and variable, under 5 kt.
- **Missing the +50 code.** 7545 is 250° at 145 kt, not a direction of 750°.
- **Adding a plus sign above 24,000 ft.** Every temperature up there is negative.
- **Using the wrong column or the wrong window.** Check the FOR USE times, and count the columns from the FT line, not from the start of the station's line, because the lowest levels may be blank.
- **Using a true wind as magnetic.** FB winds are in degrees true, like METAR and TAF winds. Convert before you work out a magnetic heading with the [true and magnetic bearing tool](/geodesy/magnetic/true-to-magnetic/). See [true vs. magnetic north](/learn/true-vs-magnetic-north/).

## Where the numbers come from

The coding rules, the Kansas City example, and the forecast periods are from the FAA *Aviation Weather Handbook* chapter on forecasts. The [winds aloft decoder](/aviation/weather/fb-winds-decode/) reads a single group, a station line, or a whole pasted product, and lines the groups up from the right so blank low levels do not shift the columns. A decoded forecast is a study aid: get a current official briefing before flight.
