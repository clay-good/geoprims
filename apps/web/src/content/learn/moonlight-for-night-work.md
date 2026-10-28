---
title: How much light the moon will give you
description: What the illuminated fraction does and does not tell you about moonlight, why the moon rises nearly an hour later each day, and how to read moonrise, moonset and phase for a night's work.
summary: "Reading the moon for night operations: the lit fraction, the 50-minute daily retardation, the dates with no moonrise at all, and why a half moon is far dimmer than half as bright."
audience: Surveyors
published: 2026-10-28
tools:
  - time.moon.rise-set
  - time.moon.phase
  - time.moon.position
sources:
  - title: Rise, Set, and Twilight Definitions
    issuer: US Naval Observatory, Astronomical Applications Department
    edition: Current web edition
    locator: Moonrise and moonset, defined by the geocentric zenith distance of the moon's centre at 90.5666 degrees plus its apparent angular radius less its horizontal parallax
    url: https://aa.usno.navy.mil/faq/RST_defs
  - title: USNO Astronomical Applications API
    issuer: US Naval Observatory, Astronomical Applications Department
    edition: API 4.0.1
    locator: The one-day rise, set, and transit endpoint and the moon-phase endpoint
    url: https://aa.usno.navy.mil/data/api
  - title: Astronomical Algorithms
    issuer: Meeus, J., Willmann-Bell
    edition: 2nd edition
    locator: Chapter 48, Illuminated Fraction of the Moon's Disk
    url: https://archive.org/details/astronomicalalgorithmsjeanmeeus1991
---

Planning a night of fieldwork, a watch, or a flight around the moon needs three different answers: when it is up, how much of it is lit, and how high it gets. They are not the same question, and the lit fraction on its own is the one most often misread.

## The lit fraction is not brightness

A full moon is not twice as bright as a half moon. It is about ten times brighter. The illuminated fraction is a geometric number: it is the share of the disc that is in sunlight, and at first quarter that share is exactly half.

But the half you can see at first quarter is lit from the side, so its craters and mountains throw long shadows across it, and the whole disc is dimmer than its area suggests. At full moon the sunlight comes from behind you, every shadow is hidden behind the thing that casts it, and the surface also reflects light preferentially straight back the way it came. The result is a sharp surge in brightness in the last couple of days before full.

So read the lit fraction as "where in the month am I", not as "how many lumens will I get". For light on the ground, a nearly full moon high in the sky is worth many times a half moon low down.

## The moon is late every day

The moon goes round the Earth in about 27 days while the Earth turns once a day, so the moon falls behind the stars and rises about 50 minutes later each day. Everything awkward about planning around it follows from that one fact.

It means moonrise walks through the clock over a month. It means the moon is up in the daytime for half of every month, which surprises people. And it means a calendar date can hold two moonrises or none at all: if the moon rises at 23:40 on one date, the next rise is after midnight on the date after, and the date in between has no moonrise.

That is not an error, and it is not a polar effect. It happens at every latitude, about once a month for rise and once a month for set. The rise and set tool says plainly when a date has none, and gives the times either side so you can see what happened.

## Inside the polar circles

Nearer the poles the moon behaves like the sun: it can stay above the horizon for days, or below it for days. The tool reports that state rather than inventing a time, which matters because a blank field and "the moon does not set today" mean very different things to someone planning a shift.

## What counts as moonrise

Moonrise is not the instant the moon's centre reaches the horizon. The US Naval Observatory defines it as the moment the geocentric zenith distance of the centre reaches 90°34′ plus the moon's apparent angular radius less its horizontal parallax. Three things are packed into that:

- **34 minutes of arc** of atmospheric refraction at the horizon, which lifts the moon into view before it is geometrically up.
- **The moon's own radius**, about 15.5 minutes of arc, because what you see appear is the upper limb, not the centre.
- **The horizontal parallax**, about 57 minutes of arc, which is the correction from the centre of the Earth, where the almanac works, to the surface, where you are standing.

The parallax is the big one and it points the other way from the first two, so the net effect is that at moonrise the moon's centre is actually a little *above* the geocentric horizon, by about 7 minutes of arc. This is why moonrise cannot be worked out the same way as sunrise, and why a tool that treats it the same way can be several minutes off.

Because the parallax and the radius both depend on the moon's distance, and the moon's orbit is noticeably eccentric, the threshold moves through the month. The tools here use the definition above at every step rather than a fixed altitude.

## How high it gets matters as much as whether it is up

A moon 5° above the horizon is behind the trees, dimmed by a long path through the atmosphere, and casting shadows the length of a field. The same moon at 60° lights the ground. Upper transit is when the moon is highest for the day, and the altitude it reaches there is the number worth planning on.

The moon's declination swings further than the sun's, between about 28° north and 28° south over a month at the extremes of its cycle, so its highest point moves a great deal more through the month than the sun's does through the year. Two full moons in consecutive months can reach very different altitudes at the same place.

## Putting it together for a night

A practical order:

1. **Rise, set and transit** for the local date, with the zone, to know whether the moon is up during your window and how high it gets.
2. **Phase** to know how much of the disc is lit and where in the month you are, and when the next quarter falls if you have a choice of dates.
3. **Position** for the specific hour, to see whether the moon clears a ridge or a building from where you will actually be standing.

The times here are for a level horizon at sea level. Terrain moves them, sometimes by a lot: a moon that rises at 20:00 by the almanac may not clear an eastern ridge until 21:30. Refraction at the horizon also varies with the weather, so a minute of disagreement with another source is normal and not worth chasing.
