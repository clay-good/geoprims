---
title: Sunrise, sunset, and twilight explained
description: What sunrise, sunset, and civil, nautical, and astronomical twilight mean, why your weather app differs by a minute or two, and the Part 107 twilight rule.
summary: How sunrise and the three twilights are defined, why published times disagree, and what they mean for flying a drone near dawn and dusk.
audience: Drone operators
published: 2026-09-23
tools:
  - time.sun.events
  - time.sun.position
  - time.sun.mapping-window
  - time.sun.aviation-nights
sources:
  - title: Rise, Set, and Twilight Definitions
    issuer: US Naval Observatory, Astronomical Applications Department
    edition: Web page, current
    locator: Sunrise and sunset (90.8333° zenith distance), civil, nautical, and astronomical twilight
    url: https://aa.usno.navy.mil/faq/RST_defs
  - title: NOAA Solar Calculator, General Solar Position Calculations
    issuer: NOAA Global Monitoring Laboratory
    edition: Web page, current
    locator: Accuracy of sunrise and sunset, refraction at 0.833°
    url: https://gml.noaa.gov/grad/solcalc/calcdetails.html
  - title: 14 CFR 107.29, Operation at night
    issuer: Federal Aviation Administration, via the Electronic Code of Federal Regulations
    edition: eCFR, current as of 2026-09-21
    locator: Paragraphs (a) to (c), night, civil twilight, and anti-collision lighting
    url: https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-107/subpart-B/section-107.29
---

Sunrise and sunset are the moments the top edge of the sun meets a flat, clear horizon. By convention that is when the center of the sun is 0.833° below the horizon: half the sun's width, plus the amount the air bends light near the ground. Twilight is the time before sunrise and after sunset when the sky is still lit. It comes in three stages, civil, nautical, and astronomical, which end when the sun's center is 6°, 12°, and 18° below the horizon.

## Why it matters

For a drone pilot the times set the rules. Under Part 107, flying in the 30 minutes after sunset or before sunrise needs anti-collision lighting, and flying at night also needs the updated training. The sun's height also decides how a mapping flight comes out: a low sun leaves long shadows and dim light.

## How the times are worked out

The US Naval Observatory defines sunrise and sunset as the moment the center of the sun is at a geometric zenith distance of 90.8333°, which is 50 arcminutes below the horizon. That 50 arcminutes is 16 for the sun's apparent radius and 34 for the average bending of light by the atmosphere at the horizon.

The method has three steps:

1. **Where the sun is.** For a date and time, work out the sun's position against the stars, then its angle north or south of the equator (declination) and its hour angle from your longitude. The [sun position tool](/time/sun/position/) uses the NREL Solar Position Algorithm for this and shows the NOAA result beside it.
2. **How high it is.** From your latitude, the declination, and the hour angle, find the sun's elevation above the horizon.
3. **When it crosses.** Search the day for the moments the elevation passes −0.833° (sunrise and sunset), −6° (civil), −12° (nautical), and −18° (astronomical).

| Event | Sun's center | What you see |
|---|---|---|
| Sunrise, sunset | 0.833° below | Top of the sun on the horizon |
| Civil twilight ends | 6° below | Lights needed for most outdoor work |
| Nautical twilight ends | 12° below | The horizon fades from view |
| Astronomical twilight ends | 18° below | Starlight outshines the sun's glow: fully dark |

## A worked example

Denver on June 21, 2026, the summer solstice, in local daylight time (UTC−6), from the [sunrise and twilight tool](/time/sun/events/):

| Event | Morning | Evening |
|---|---|---|
| Astronomical twilight | 03:30 | 22:34 |
| Nautical twilight | 04:18 | 21:46 |
| Civil twilight | 05:00 | 21:04 |
| Sunrise, sunset | **05:32** | **20:31** |

Morning times are when each stage begins, and evening times when it ends. Solar noon is 13:02 and the day is 14 h 59 min long. Civil twilight runs 32 minutes in the morning and 33 in the evening. Solar noon falls after 1 pm because Denver keeps daylight saving time in summer.

Farther north the stages stretch. In Anchorage on the same date, the sun rises at 04:20 and sets at 23:43 local, and it never gets 6° below the horizon, so civil twilight lasts all night.

## Why your weather app says something different

Two sources can disagree by a minute or more, and both can be reasonable.

- **Rounding.** Some round to the nearest minute, some cut off the seconds.
- **Place.** A city's time is for one point. Every 1° of longitude moves sunrise by about 4 minutes, so a site 30 miles east or west can be 2 minutes off.
- **Height.** From above the surrounding ground you see past the normal horizon. With an observer height of 120 m, Denver's sunrise moves to 05:30 and sunset to 20:33, and the day gains 4 minutes.
- **Terrain.** A mountain ridge to the west hides the sun well before the calculated sunset.
- **The air.** The 34-arcminute bend is an average. The USNO says real rise and set times cannot be computed exactly, because refraction at the horizon changes with the weather.

NOAA gives its sunrise and sunset results as good to within a minute between 72° north and south, and within 10 minutes closer to the poles, where the sun crosses the horizon at a shallow angle.

## Part 107 and civil twilight

For Part 107, §107.29(c) does not use the 6° definition. Outside Alaska, civil twilight is a fixed period: from 30 minutes before official sunrise to sunrise, and from sunset to 30 minutes after. In Alaska it is the Air Almanac's civil twilight.

In Denver on June 21 that makes the evening twilight 20:31 to 21:01, while the sun reaches 6° below at 21:04. During Part 107 civil twilight, and at night, the drone needs anti-collision lighting visible for at least 3 statute miles. At night the remote pilot also needs to have passed the knowledge test or training under §107.65 after April 6, 2021. For the manned-aircraft rules, which use other definitions of night, see [when is it night in aviation](/learn/night-in-aviation/).

## Golden hour and mapping light

Golden hour has no official definition. Photographers use it for the time when the sun is low, and a common working limit is 6° above the horizon. Setting the [mapping light window](/time/sun/mapping-window/) to 6° in Denver on June 21 shows the sun above 6° from 06:13 to 19:51, so the evening's low light runs from 19:51 to sunset at 20:31.

Mapping wants the opposite: a high sun and short shadows. At the default 30° threshold, the same day's window is 08:23 to 17:40, 9 h 17 min, with the sun peaking at 73.7°. The [sun position tool](/time/sun/position/) shows the difference. A 10 m object casts a 2.92 m shadow at solar noon, when the sun is 73.7° up, and a 37 m shadow at 19:00, when it is 15.1° up.

## Common mistakes

- **Treating sunset as dark.** In Denver, civil twilight lasts from 27 minutes at the equinoxes to 33 minutes at the June solstice, and the Part 107 lighting rule applies from sunset.
- **Using the 6° twilight for Part 107.** Outside Alaska the rule is a flat 30 minutes.
- **Using a city's times for a distant site.** Enter the site's own latitude and longitude.
- **Forgetting daylight saving time.** The tools take a UTC offset. Denver is UTC−6 in summer and UTC−7 in winter.
- **Expecting a normal night in the far north.** In summer at high latitude, twilight may never end.

## Where the numbers come from

The definitions are the US Naval Observatory's. The solar position follows the NREL Solar Position Algorithm, cross-checked against NOAA's equations. The [sunrise and twilight tool](/time/sun/events/) gives every event in local time and Zulu, and says so when the sun never rises, never sets, or twilight never ends. The Part 107 rule follows the current eCFR text of 14 CFR 107.29.
