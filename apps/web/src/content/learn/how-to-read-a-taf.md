---
title: How to read a TAF
description: A TAF is an airport forecast in code. How the valid period, FM, TEMPO, PROB30, and BECMG groups work, with a Denver TAF decoded into a timeline.
summary: The valid period and each change group of a terminal forecast, and how to turn one into a timeline of what to expect and when.
audience: Pilots
published: 2026-09-23
tools:
  - aviation.weather.taf-decode
  - aviation.weather.metar-decode
  - time.scale.utc-offset
sources:
  - title: Aviation Weather Handbook, Chapter 27, Forecasts
    issuer: Federal Aviation Administration (posted by the FAA Safety Team)
    edition: FAA-H-8083-28 series, 2024 posting
    locator: Section 27.3, Terminal Aerodrome Forecast (valid period, FM, TEMPO, PROB30, joint-use airports)
    url: https://www.faasafety.gov/files/events/SO/SO15/2024/SO15129447/FAA-H-8083-28Chpt27.pdf
  - title: Key to Aerodrome Forecast (TAF) and Aviation Routine Weather Report (METAR)
    issuer: Federal Aviation Administration and National Weather Service
    edition: Key card, as posted by NWS New York
    locator: Front (valid period) and back (FM, TEMPO, PROB, BECMG, notes on NWS TAFs)
    url: https://www.weather.gov/media/okx/Aviation/TAF_Card.pdf
---

A TAF (terminal aerodrome forecast) is the forecast for one airport, written in the same code as a METAR. It covers the area within 5 statute miles of the center of the runways for 24 or 30 hours. The forecast is split into periods by change groups: FM starts a new set of conditions, TEMPO and PROB30 add brief or possible changes on top of them, and BECMG marks a gradual change.

## Why it matters

The METAR tells you what the weather is now. The TAF tells you what it should be when you get there, and whether an alternate is likely to be needed. Reading one well means turning its lines into a timeline: what the prevailing conditions are at each hour, and what might happen inside them.

## The header

`TAF KDEN 181720Z 1818/1918`

- **TAF** is a routine forecast. TAF AMD is an amended one. An amendment replaces the old forecast as soon as it is issued, even if its valid period starts later.
- **KDEN** is the airport.
- **181720Z** is when it was issued: the 18th at 1720Z. Routine TAFs go out 20 to 40 minutes before their valid period starts.
- **1818/1918** is the valid period: from the 18th at 1800Z to the 19th at 1800Z. Each half is a day of the month and an hour in UTC. An end of 24 means midnight at the end of that day.

In the US, the National Weather Service issues routine TAFs four times a day, at 0000Z, 0600Z, 1200Z, and 1800Z. Most cover 24 hours. Some large airports get 30 hours.

## The change groups

| Group | What it means |
|---|---|
| FM190200 | From the 19th at 0200Z, a new, complete forecast replaces everything before it |
| TEMPO 1820/1824 | Temporary changes between those hours, each lasting an hour or less and together covering less than half the window |
| PROB30 1914/1918 | A 30% chance of the conditions that follow, in that window |
| BECMG 1910/1912 | A gradual change that starts in that window and holds after it |

An FM group is self-contained. It repeats the wind, visibility, and clouds even when they do not change, so it stands on its own. A TEMPO or PROB30 group lists only what changes. Anything it leaves out stays as the prevailing forecast has it.

The handbook says TEMPO conditions have a better than even chance of happening. PROB30 is for a lower-probability thunderstorm or precipitation event. NWS TAFs use only PROB30, never in the first 9 hours, while military and international TAFs may also use PROB40. The FAA and NWS key card notes that NWS TAFs do not use BECMG, but you will see it in TAFs from military and international airports.

## A worked example

Here is a Denver TAF, decoded by the [TAF decoder](/aviation/weather/taf-decode/) with Denver's summer offset of UTC−6. The example includes a BECMG group to show how one reads.

`TAF KDEN 181720Z 1818/1918 30012G22KT P6SM SCT080 BKN200 TEMPO 1820/1824 VRB25G35KT 3SM TSRA BKN060CB FM190200 32008KT P6SM FEW100 BECMG 1910/1912 18010KT PROB30 1914/1918 3SM -SHRA BKN030`

| Period | UTC | Starts, local | Forecast | Category |
|---|---|---|---|---|
| Base | 18th 1800Z to 19th 0200Z | 12:00 | 300° true at 12 kt, gusting 22 kt; more than 6 SM; scattered at 8,000 ft, broken at 20,000 ft | VFR |
| TEMPO | 18th 2000Z to 2400Z | 14:00 | Variable at 25 kt, gusting 35 kt; 3 SM; thunderstorm with rain; broken cumulonimbus at 6,000 ft | MVFR |
| FM | 19th 0200Z to 1800Z | 20:00 on the 18th | 320° true at 8 kt; more than 6 SM; few at 10,000 ft | VFR |
| BECMG | 19th 1000Z to 1200Z | 04:00 | Wind turning to 180° true at 10 kt | |
| PROB30 | 19th 1400Z to 1800Z | 08:00 | 3 SM; light rain showers; broken at 3,000 ft | MVFR |

The decoder sums it up as a forecast from the 18th at 1800Z to the 19th at 1800Z in 5 periods.

Read as a timeline, it says: a gusty but VFR afternoon, with thunderstorms coming and going between 2 pm and 6 pm local that bring gusts to 35 kt, visibility down to 3 SM, and a 6,000 ft ceiling in cumulonimbus. From 8 pm the storms are gone and the wind is light. Early the next morning the wind turns to the south. After 8 am there is a 30% chance of showers with a 3,000 ft ceiling.

Two things are easy to miss. The FM period starts at 0200Z on the 19th, which is still the evening of the 18th in Denver. And the BECMG group changes only the wind: the visibility and clouds carry on from the FM group before it.

## Common mistakes

- **Misreading the time groups.** FM100000 is 0000Z on the 10th, not 1000Z. The key card warns about exactly this.
- **Forgetting UTC.** Every time in a TAF is UTC. Convert to local with the [UTC offset tool](/time/scale/utc-offset/), and watch for the date change.
- **Treating TEMPO as a footnote.** TEMPO conditions are more likely than not to happen. Plan for them.
- **Dropping what a TEMPO or BECMG leaves out.** Elements not repeated stay as the prevailing forecast has them.
- **Reading BECMG as a sudden change.** The change happens at some point in the window and holds after it.
- **Using an old TAF after an amendment.** TAF AMD replaces the earlier forecast at once.
- **Mixing up true and magnetic wind.** TAF winds are true, like the METAR. The tower gives magnetic. See [true vs. magnetic north](/learn/true-vs-magnetic-north/).

## Where the numbers come from

The meanings of the header and change groups follow the FAA *Aviation Weather Handbook* chapter on forecasts and the FAA and NWS key to TAF and METAR codes. The flight categories are the ones described in [how to read a METAR](/learn/how-to-read-a-metar/). The [TAF decoder](/aviation/weather/taf-decode/) turns any TAF you paste into this kind of timeline, with local times when you give an offset, and flags a change group that falls outside the valid period. To compare the forecast with what is happening now, paste the current report into the [METAR decoder](/aviation/weather/metar-decode/). A decoded forecast is a study aid: get a current official briefing before flight.
