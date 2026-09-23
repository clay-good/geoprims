---
title: When is it night in aviation?
description: US aviation rules use four different nights. When each one starts, which regulation it comes from, and what counts for night passenger currency.
summary: Position lights, logging night, passenger currency, and Part 107 twilight each start at a different time. Here is when, and why.
audience: Pilots
published: 2026-09-23
tools:
  - time.sun.aviation-nights
  - time.sun.night-currency
  - time.sun.events
sources:
  - title: 14 CFR 1.1, General definitions (night)
    issuer: Federal Aviation Administration, via the Electronic Code of Federal Regulations
    edition: eCFR, current as of 2026-09-21
    locator: Definition of night
    url: https://www.ecfr.gov/current/title-14/chapter-I/subchapter-A/part-1/section-1.1
  - title: 14 CFR 61.57, Recent flight experience, pilot in command
    issuer: Federal Aviation Administration, via the Electronic Code of Federal Regulations
    edition: eCFR, current as of 2026-09-21
    locator: Paragraph (b), night takeoff and landing experience
    url: https://www.ecfr.gov/current/title-14/chapter-I/subchapter-D/part-61/subpart-A/section-61.57
  - title: 14 CFR 91.209, Aircraft lights
    issuer: Federal Aviation Administration, via the Electronic Code of Federal Regulations
    edition: eCFR, current as of 2026-09-21
    locator: Paragraph (a), sunset to sunrise
    url: https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-91/subpart-C/section-91.209
  - title: 14 CFR 107.29, Operation at night
    issuer: Federal Aviation Administration, via the Electronic Code of Federal Regulations
    edition: eCFR, current as of 2026-09-21
    locator: Paragraphs (a) to (c), night, civil twilight, and anti-collision lighting
    url: https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-107/subpart-B/section-107.29
---

In US aviation rules, "night" does not start at one time. Position lights go on at sunset. Night flight time for your logbook starts at the end of evening civil twilight, when the sun is 6° below the horizon. Landings that count toward night passenger currency start one hour after sunset. Drone pilots under Part 107 have their own 30-minute civil twilight. On a summer evening in Denver, those four start times spread across an hour.

## Why it matters

Each rule asks a different question. Do I need my lights on? Can I log this as night? Does this landing keep me current to carry passengers at night? Does my drone need anti-collision lighting? A landing can count for one and not another. Passenger currency is the easy one to get wrong: a landing just after dark goes in the logbook as night, but it does not count toward the three you need.

## The four nights

| Rule | Starts | Ends | What it controls |
|---|---|---|---|
| 14 CFR 91.209(a) | Sunset | Sunrise | Position lights on |
| 14 CFR 1.1 | End of evening civil twilight | Beginning of morning civil twilight | Night for logging and most other rules |
| 14 CFR 61.57(b) | 1 hour after sunset | 1 hour before sunrise | Takeoffs and landings for night passenger currency |
| 14 CFR 107.29(c) | Sunset, and 30 minutes before sunrise | 30 minutes after sunset, and sunrise | Part 107 civil twilight |

**Night, by definition.** Section 1.1 defines night as the time between the end of evening civil twilight and the beginning of morning civil twilight, as published in the Air Almanac and converted to local time. Civil twilight ends when the center of the sun is 6° below the horizon. This is the night you log under 14 CFR 61.51, and the one most rules mean when they say "night."

**Lights.** Section 91.209 requires lighted position lights from sunset to sunrise. Alaska has its own wording, tied to how far you can see an unlit object and to the sun being more than 6° below the horizon.

**Passenger currency.** Section 61.57(b) says you may not act as pilot in command carrying passengers from 1 hour after sunset to 1 hour before sunrise unless, in the preceding 90 days, you made three takeoffs and three landings to a full stop in that same period. They must be in the same category, class, and type (if a type rating is required).

**Part 107.** Since April 2021, drone pilots may fly at night under Part 107 after the updated knowledge test or training under §107.65, with anti-collision lighting visible for 3 statute miles. The same lighting is required during civil twilight. For this rule only, §107.29(c) defines civil twilight as a fixed 30 minutes before official sunrise and after official sunset, except in Alaska, where it is the Air Almanac period.

## A worked example

Denver on June 21, 2026, with a landing at 9:20 pm local (UTC−6), run through the [four aviation nights tool](/time/sun/aviation-nights/):

| Period | From | To |
|---|---|---|
| Position lights | 20:31 | 05:32 |
| Part 107 civil twilight, evening | 20:31 | 21:01 |
| Logging night | 21:04 | 05:00 |
| Passenger currency night | 21:31 | 04:32 |
| Part 107 civil twilight, morning | 05:02 | 05:32 |

The tool's verdict: "Night landings for passenger currency count after 21:31 local. This landing is loggable night but does not count for currency."

The landing at 21:20 comes 16 minutes after civil twilight ends, so it is night for the logbook. It is 11 minutes too early to count toward passenger currency.

Notice the small gap between 21:01 and 21:04. Part 107's twilight is a fixed 30 minutes, but on this date the sun took 33 minutes to reach 6° below the horizon. In Denver it takes 27 minutes at the September equinox (sunset 18:57, civil twilight ends 19:24) and 30 minutes at the December solstice (16:39 and 17:09). So the Part 107 twilight can end a few minutes before or after the 1.1 night begins. With anti-collision lighting on for both twilight and night, a drone pilot is covered either way.

## Staying current

The [night currency tool](/time/sun/night-currency/) takes logbook entries and checks each one against that night's own sunset and sunrise. Three night takeoffs and full-stop landings in a single-engine land airplane near Denver, on May 1, May 10, and June 2, 2026, all late in the evening:

| Landing | Currency window that night | Counts? |
|---|---|---|
| May 1 at 22:30 | 20:53 to 04:58 | Yes |
| May 10 at 22:30 | 21:02 to 04:48 | Yes |
| June 2 at 23:00 | 21:22 to 04:32 | Yes |

Checked on July 15, 2026, the tool's verdict is "You are current to carry passengers at night through 2026-07-30." That date is 90 days after the oldest of the three landings. After it, only two remain inside the 90 days, and on July 31 the tool answers "You are not current to carry passengers at night on that date." Add one more qualifying night takeoff and landing on July 20 and currency runs through 2026-08-08, 90 days after the May 10 landing.

Notice how the currency window moves. It starts 29 minutes later at the start of June than at the start of May, because sunset moves later as summer comes on.

## Common mistakes

- **Counting a dusk landing toward currency.** Loggable night starts about half an hour after sunset. Currency landings start a full hour after.
- **Using a fixed clock time.** Sunset in Denver moves by more than three hours through the year, so "after 9 pm" is right in some months and wrong in others.
- **Counting touch-and-goes.** Section 61.57(b) needs landings to a full stop.
- **Mixing up categories and classes.** Landings in a single-engine airplane do not keep you current in a multiengine one.
- **Assuming the Part 107 twilight and 1.1 twilight match.** They are close, but not always the same minute, and Alaska follows its own rules.

## Where the numbers come from

The rules follow the current eCFR text of 14 CFR 1.1, 61.57, 91.209, and 107.29. The sun times come from the same solar calculation as the [sunrise and twilight tool](/time/sun/events/): sunrise and sunset when the sun's center is 0.833° below the horizon, and civil twilight at 6° below. The legal source for twilight is the Air Almanac, and the [four aviation nights tool](/time/sun/aviation-nights/) warns that its computed times can differ from it by a minute. For the difference between sunset, twilight, and dark, see [sunrise, sunset, and twilight](/learn/sunrise-and-twilight/). Currency is your responsibility as pilot in command: these tools are a planning aid, not legal advice.
