---
title: Zulu time (UTC) explained
description: Zulu time is UTC, the one clock aviation runs on. How to convert local time to Zulu, how daylight saving time changes the offset, and the traps at midnight.
summary: What Zulu time is, why weather reports and flight plans use it, and how to convert local time to UTC and back without the daylight saving slip.
audience: Pilots
published: 2026-09-23
tools:
  - time.scale.utc-offset
  - time.scale.zone-info
  - time.scale.block-time
  - time.scale.decimal-hours
sources:
  - title: Aeronautical Information Manual
    issuer: Federal Aviation Administration
    edition: Current edition
    locator: Chapter 4, Section 2, paragraph 4-2-12, Time (TBL 4-2-12, Standard Time to Coordinated Universal Time)
    url: https://www.faa.gov/air_traffic/publications/atpubs/aim_html/chap4_section_2.html
  - title: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
    issuer: Federal Aviation Administration
    edition: 2023
    locator: Chapter 16, Navigation (time zones, pages 16-4 and 16-5), and Chapter 13, Aviation Weather Services (METAR date and time)
    url: https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/phak
  - title: Daylight Saving Time
    issuer: National Institute of Standards and Technology
    edition: Web page, rules for 2026
    locator: Current US rules, 2026 dates, and places that do not observe DST
    url: https://www.nist.gov/pml/time-and-frequency-division/popular-links/daylight-saving-time-dst
  - title: Time Zone Database (tzdb)
    issuer: Internet Assigned Numbers Authority
    edition: Release 2026d
    locator: Zone rules for every region, used by the tools on this page
    url: https://www.iana.org/time-zones
---

Zulu time is Coordinated Universal Time (UTC), the single clock that aviation uses everywhere. "Zulu" is the phonetic word for Z, the letter written after a UTC time, as in 1905Z. To convert local time to Zulu, add your time zone's offset from UTC, and one hour less while daylight saving time is in effect.

The FAA's *Aeronautical Information Manual* says it plainly: the FAA uses UTC for all operations. Weather reports, forecasts, NOTAMs, flight plans, and ATC times are all in Zulu unless someone says "local."

## Why aviation uses UTC

A flight can cross several time zones, and weather and air traffic data come from stations all over the world. If every report used its own local time, you would have to know each station's zone and daylight saving rule just to tell which report is newer. With UTC, a METAR stamped 1853Z in Chicago and one stamped 1853Z in Denver were issued at the same moment. The *Pilot's Handbook of Aeronautical Knowledge* describes it as the standard that puts the whole world on one time.

UTC is also written on a 24-hour clock: 0000 to 2359, with no a.m. or p.m. to confuse.

## How to convert local time to Zulu

The AIM gives the US offsets from standard time:

| Time zone | Standard time to UTC |
|---|---|
| Eastern | Add 5 hours |
| Central | Add 6 hours |
| Mountain | Add 7 hours |
| Pacific | Add 8 hours |
| Alaska | Add 9 hours |
| Hawaii | Add 10 hours |

For daylight time, subtract 1 hour from those figures. So Eastern Daylight Time is 4 hours behind UTC, and Central Daylight Time is 5.

In 2026, US daylight saving time runs from March 8 to November 1, starting and ending at 2 a.m. local time. Not everywhere observes it. Hawaii and most of Arizona stay on standard time all year, as do Puerto Rico, Guam, the US Virgin Islands, and American Samoa.

The [local time to UTC tool](/time/scale/utc-offset/) does the conversion with the time zone rules from the IANA time zone database. Give it a place name like America/Chicago and it finds the right offset for that date, daylight saving or not.

## A worked example

14:05 local in Chicago on July 1, 2026:

| Item | Result |
|---|---|
| Zone and abbreviation | America/Chicago, CDT |
| Offset from UTC | -05:00 |
| UTC | 2026-07-01T19:05:00Z |
| Zulu time | **1905Z** |

Chicago is on Central Daylight Time in July: 6 hours for Central, less 1 for daylight time, is 5. The same 14:05 on January 15 is Central Standard Time, 6 hours behind, and the tool gives 2005Z.

More cases from the tool:

| Local time | Zone | Zulu | Note |
|---|---|---|---|
| 14:05, July 1 | Phoenix (MST, -07:00) | 2105Z | Arizona stays on standard time |
| 20:30, July 1 | Los Angeles (PDT, -07:00) | 0330Z | The UTC date is July 2, the next day |
| 01:30, November 1 | New York (EDT, -04:00) | 0530Z | Happens twice; the second is 0630Z |
| 02:30, March 8 | New York | none | That time does not exist |

The last two rows are the daylight saving changes. On March 8, New York clocks jump from 2 a.m. to 3 a.m., so 02:30 never happens; the tool rejects it rather than guess. On November 1, clocks fall back from 2 a.m. to 1 a.m., so 01:30 happens twice, first on daylight time and then on standard time. The tool uses the first and names both.

## Going the other way

From Zulu back to local, subtract the offset. The tool's UTC-to-local direction gives 2300Z on July 1 in Anchorage as 15:00 local, Alaska Daylight Time, 8 hours behind. To see which offset a place is on today and when it next changes, the [time zone rules tool](/time/scale/zone-info/) shows it: at 0000Z on September 18, 2026, Denver is on MDT, 6 hours behind UTC, until 2026-11-01T08:00:00Z, when it moves to 7 hours behind.

## Times in your logbook

Block times often cross midnight in Zulu. The [block time tool](/time/scale/block-time/) handles that: out at 2215Z and in at 0140Z is 3 h 25 min, or 3.42 hours. Decimal hours can trip you up too: 1.3 hours is 1 h 18 min, not 1 h 30 min, as the [decimal hours tool](/time/scale/decimal-hours/) shows.

## Common mistakes

- **Forgetting daylight saving time.** The AIM table is for standard time. In summer, the offset is one hour smaller.
- **Missing the date change.** An evening departure on the West Coast is already the next day in UTC. Flight plans, NOTAMs, and TAF periods use the UTC date.
- **Applying daylight saving where there is none.** Most of Arizona and all of Hawaii do not change their clocks.
- **Reading a METAR time as local.** The time group in a METAR, like 161753Z, is the day of the month and the time in UTC.
- **Using a fixed offset for a whole trip.** A long flight can cross zones, and a date can cross a daylight saving change. Convert each time by itself.

## Where the numbers come from

The US offsets are from AIM paragraph 4-2-12, and the 2026 daylight saving dates and exceptions from NIST. The tools use the IANA time zone database, release 2026d. Every conversion above comes from the [local time to UTC tool](/time/scale/utc-offset/). This is a planning and education aid; for official times, use your flight plan, ATC, and published sources.
