---
title: How a sextant sight becomes a line of position
description: "What sight reduction does, step by step: the almanac data for the second of the sight, the corrections that turn the altitude you read into the observed altitude, and the intercept that puts the line on the chart."
summary: The three steps from a sextant reading to a plotted line, what each correction is for, and why the clock matters more than the sextant.
audience: Mariners
published: 2026-10-28
tools:
  - navigation.celestial.sight-reduction
  - navigation.celestial.altitude-correction
  - navigation.celestial.almanac
sources:
  - title: The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume I
    issuer: National Geospatial-Intelligence Agency
    edition: 2024 edition
    locator: Chapter 16, Sextant Altitude Corrections; Chapter 20, Sight Reduction, and Appendix B
    url: https://msi.nga.mil/Publications/APN
  - title: USNO Astronomical Applications API
    issuer: US Naval Observatory, Astronomical Applications Department
    edition: API 4.0.1
    locator: The celestial-navigation endpoint, which gives the hour angle, declination, computed altitude, and azimuth at an assumed position
    url: https://aa.usno.navy.mil/data/api
  - title: The Calculation of Astronomical Refraction in Marine Navigation
    issuer: Bennett, G. G., The Journal of Navigation (Royal Institute of Navigation)
    edition: Vol. 35, No. 2
    locator: pp. 255-259, the refraction formula these tools use
    url: https://doi.org/10.1017/S0373463300022037
---

A sextant does not measure where you are. It measures how high a body is above the horizon, and that single number puts you somewhere on a circle drawn on the Earth. Sight reduction is the arithmetic that turns the reading into a line you can draw on a chart.

## What one sight actually tells you

Every body in the sky is directly over one point on the Earth at any instant: its geographic position. If you measure the body 40° above the horizon, you are 50° of arc away from that point, in some direction. That puts you on a circle centred on the geographic position, with a radius of 50° of arc. One sight, one circle.

The circle is far too big to draw. So a navigator draws the short piece of it that passes nearby, and over the few miles that matter, a straight line is close enough. Getting from the sextant reading to that line is three steps.

## Step one: what the sky was doing

The almanac gives the body's place for the second of the sight: its Greenwich hour angle, which is how far west of Greenwich it stood, and its declination, which is how far north or south of the equator. Those two numbers are its geographic position.

This is where the clock comes in, and it is the part people underestimate. The sky turns 15° of hour angle every hour, which is 15 minutes of arc every minute of time. A clock a minute fast moves your line about 15 nautical miles. A sextant read carelessly to the nearest two minutes of arc costs you two miles. The watch is the instrument that decides the quality of the sight.

## Step two: from the arc to the observed altitude

What you read off the sextant's arc is not the altitude of the body's centre above the true horizon. Five things stand between them, and the correction tool shows each one on its own line.

- **Index correction.** The sextant's own error, found by checking it against the horizon. It belongs to the instrument, not the sight.
- **Dip.** From any height above the water you see past the true horizon, so every body looks higher than it is. The correction is about 1.76 minutes of arc for the square root of your height of eye in metres: from 38 feet, about 5.4 minutes.
- **Refraction.** The atmosphere bends light downward, which again lifts the body. It is about 34 minutes of arc at the horizon and under one minute above 30°, and it varies with the air's temperature and pressure, which is why those are inputs.
- **Semidiameter.** On the Sun and the Moon you touch a limb, not the centre, so half the body's width is added or taken off.
- **Parallax.** The almanac's places are for the centre of the Earth. The Moon is close enough that standing on the surface instead moves it by up to about a degree, which has to be put back.

Below about 5° of altitude refraction grows fast and becomes unpredictable, which is why Bowditch advises against sights that low. The tool says so when you give it one.

## Step three: the intercept

Now you have the observed altitude, Ho. Pick a position you think you are near, usually the dead-reckoning position, and work out what altitude a body at that hour angle and declination *would* have there. That is the computed altitude, Hc, and it comes straight out of the navigational triangle.

The difference is the whole trick. If Ho is greater than Hc, you are closer to the body's geographic position than your assumed position is, by one nautical mile for every minute of arc of difference. If Ho is less, you are further away. That difference is the intercept, and the direction to move is the body's azimuth, which the same triangle gives.

So: start at the assumed position, go the intercept's distance along the azimuth toward the body (or along its reciprocal, away), and draw a line square to the azimuth through that point. You are somewhere on that line.

## Why it is a line and not a fix

One sight gives one line. Two or three sights of different bodies, taken within a few minutes of each other, give lines that cross, and the crossing is a fix. Sights taken an hour apart need the earlier lines advanced for the run between them, which is the running fix.

The method has a limit worth knowing. Replacing the circle with its tangent is an approximation, and it gets worse as the altitude rises, because a high body's circle is small and curves sharply. Above about 70° Bowditch advises working from the dead-reckoning position and keeping the line short, or plotting the circle itself.

## What these tools do and do not do

The tools here compute the body's place from published astronomical series rather than reading tabulated pages, so they are not limited to one year's almanac. They are checked against the US Naval Observatory's own data and against Bowditch's worked examples, and the agreement is stated on each tool's page.

They are a planning and teaching aid. The printed almanac and the published sight reduction tables govern, and nothing here is a substitute for knowing how to work a sight by hand when the batteries die.
