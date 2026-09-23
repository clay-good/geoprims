---
title: The wind triangle explained
description: How the wind triangle gives heading, groundspeed, and wind correction angle from course, true airspeed, and wind, what the E6B does, and where the mental shortcuts drift.
summary: Heading, groundspeed, and wind correction angle from your course, airspeed, and the wind, worked exactly and by rule of thumb.
audience: Pilots
published: 2026-09-23
tools:
  - aviation.wind.heading-groundspeed
  - aviation.wind.find-wind
  - aviation.wind.course-from-heading
  - aviation.wind.heading-chain
sources:
  - title: Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C)
    issuer: Federal Aviation Administration
    edition: 2023
    locator: Chapter 16, Navigation (flight computers, wind triangle or vector analysis, pages 16-12 to 16-16)
    url: https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/phak
  - title: Aviation Weather Handbook (FAA-H-8083-28B)
    issuer: Federal Aviation Administration
    edition: 2026
    locator: Section 3.4.2.2, Products with Wind Information (winds aloft referenced to true north)
    url: https://www.faa.gov/sites/faa.gov/files/FAA-H-8083-28B.pdf
---

The wind triangle is the drawing that shows how the wind changes your path over the ground. One side is your airplane moving through the air, on its heading at true airspeed. One side is the air moving over the ground, the wind. The third side is the result: your track and groundspeed. Solve it and you get the heading to fly and the groundspeed you will make.

The *Pilot's Handbook of Aeronautical Knowledge* calls it the basis of dead reckoning. It is what the wind side of an E6B flight computer solves, and what a GPS navigator shows you after the fact.

## The words

| Term | Meaning |
|---|---|
| True course | The line you want to follow over the ground, measured from true north |
| True airspeed (TAS) | Your speed through the air |
| Wind | The direction it blows from, true, and its speed |
| True heading | Where the nose points to hold the course |
| Wind correction angle (WCA) | The angle between heading and course: the crab |
| Groundspeed | Your speed over the ground along the course |

Winds aloft forecasts are given from true north. That is why the triangle is worked in true degrees, and variation and deviation come after.

## Why it matters

Groundspeed sets your time en route and so your fuel burn. The wind correction angle keeps you on the course you drew. Get either wrong and you arrive late, low on fuel, or somewhere else. The same geometry runs backward in flight: from the heading you hold and the track and groundspeed you see, you can work out the real wind.

## How it is worked out

The triangle is exact trigonometry, which is why a spinning computer or a calculator can solve it.

1. **Split the wind against the course.** The part across the course is the crosswind. The part along it is headwind or tailwind.
2. **Wind correction angle.** Turn into the wind until the airplane's sideways speed cancels the crosswind: sin(WCA) = crosswind ÷ TAS.
3. **Groundspeed.** Your speed along the course is TAS × cos(WCA), less the headwind or plus the tailwind.
4. **Heading.** True heading = true course + WCA, to the right if the wind is from the right.

The [heading and groundspeed tool](/aviation/wind/heading-groundspeed/) does these steps exactly and shows each one under "How we got this."

## A worked example

True course 090°, TAS 120 kt, wind 030° at 20 kt:

| Item | Result |
|---|---|
| Crosswind on the course | 17.3 kt from the left |
| Headwind on the course | 10 kt |
| Wind correction angle | 8.3° left |
| True heading | **81.7°** |
| Groundspeed | **108.7 kt** |

Notice the groundspeed: 11.3 kt below TAS, although the headwind part of the wind is only 10 kt. Pointing the nose 8.3° off the course costs a little speed along it.

The handbook's own example checks the method. It draws the triangle for a true course of 090°, a wind of 045° at 40 kt, and 120 kt TAS, and measures a true heading of 076°, a wind correction of 14°, and a groundspeed of 88 kt. The tool gives 76.4°, 13.6°, and 88.3 kt: the same answer, to the width of a pencil line.

## Rules of thumb, and how far they drift

Two shortcuts carry most pilots through a flight without a computer.

- **Maximum drift.** Divide TAS by 60. Wind speed divided by that figure is the most drift that wind can cause, when it blows straight across. At 120 kt, TAS ÷ 60 is 2, so a 40 kt wind gives at most 20°.
- **Wind correction angle.** Apply the same division to the crosswind part only: WCA ≈ crosswind ÷ (TAS ÷ 60).

How they compare with the exact triangle at 120 kt TAS:

| Case | Crosswind | Rule of thumb WCA | Exact WCA | Exact groundspeed |
|---|---|---|---|---|
| Wind 030° at 20 kt | 17.3 kt | 8.7° | 8.3° | 108.7 kt |
| Wind 045° at 40 kt | 28.3 kt | 14.1° | 13.6° | 88.3 kt |
| Wind 000° at 40 kt, straight across | 40 kt | 20° | 19.5° | 113.1 kt |
| Wind 270° at 20 kt, straight behind | 0 kt | 0° | 0° | 140 kt |

The rule reads a little high, by about half a degree in these cases, which is well inside what you can hold by hand. It grows as the wind becomes a larger share of your airspeed. For groundspeed, "TAS minus headwind" is close for small crab angles and too high for large ones: a 40 kt wind straight across still costs 6.9 kt.

## Running the triangle other ways

The same three sides can be solved for any missing piece.

- **Find the wind in flight.** Hold a heading, read your track and groundspeed from GPS, and the [find the wind tool](/aviation/wind/find-wind/) gives the wind. With the example's heading of 81.7°, TAS of 120 kt, track of 090°, and groundspeed of 108.7 kt, it returns the wind from 30° at 20 kt, back where we started.
- **See where a heading takes you.** Given a heading and the wind, the [course from heading tool](/aviation/wind/course-from-heading/) gives the track and groundspeed. Heading 81.7° in the example wind makes good a course of 90° at 108.7 kt, with 8.3° of drift.
- **Go from true course to the compass.** The [true course to compass heading tool](/aviation/wind/heading-chain/) walks the chain: true course, wind correction, variation, then deviation from your compass card.

## Common mistakes

- **Mixing true and magnetic.** Work the triangle with a true course and a true wind. Apply variation afterward.
- **Using indicated airspeed.** The triangle needs true airspeed. At altitude it can be well above indicated.
- **Correcting the wrong way.** Crab into the wind. A wind from the left means a heading to the left of the course.
- **Treating the forecast wind as the real one.** Winds aloft are a forecast. Check your groundspeed early and rework the triangle if it is off.

## Where the numbers come from

The method is the vector triangle in Chapter 16 of the FAA's *Pilot's Handbook of Aeronautical Knowledge*, solved with trigonometry instead of a protractor. Every value above comes from the [heading and groundspeed tool](/aviation/wind/heading-groundspeed/) and its sibling tools. For the same geometry at the runway, read [How to calculate crosswind](/learn/crosswind-components/). This is a planning and education aid; it does not replace your navigation equipment or charts.
