---
title: Why overlap changes heights
description: How end lap sets the air base between photos, why the base-to-height ratio controls how precisely a stereo pair measures heights, and what that means for drone mapping overlap.
summary: End lap, the air base, the base-to-height ratio, and the height precision of one stereo pair.
audience: Drone operators
published: 2026-10-19
tools:
  - drone.photogrammetry.stereo-base
  - drone.photogrammetry.parallax-height
  - drone.photogrammetry.photo-scale
  - drone.photogrammetry.trigger-distance
sources:
  - title: EM 1110-1-1000, Photogrammetric Mapping, Chapter 10, Principles of Photogrammetry
    issuer: U.S. Army Corps of Engineers
    edition: 31 March 1993 (superseded by the 2015 edition)
    locator: Equations 10-9 and 10-11 (coverage and air base) and 10-14 and 10-15 (the parallax equations)
    url: https://www.lpl.arizona.edu/hamilton/sites/lpl.arizona.edu.hamilton/files/courses/ptys551/Principles_of_Photogrammetry.pdf
  - title: "GEOG 892: Geospatial Applications of Unmanned Aerial Systems, Lesson 4, Designing a Flight Route"
    issuer: Abdullah, Q., Penn State College of Earth and Mineral Sciences
    edition: Online course text, retrieved 2026-10-19
    locator: Airbase from image coverage and end lap
    url: https://courses.ems.psu.edu/geog892/node/658
---

More end lap moves the photos closer together, and photos that are closer together see the ground from nearly the same place. That makes each stereo pair worse at measuring heights. The extra photos still help mapping software in other ways, but the geometry of any one pair gets weaker.

The number that captures this is the base-to-height ratio: the distance between two exposures (the air base) divided by the flying height above the ground.

## Why it matters

Heights in photogrammetry come from parallax. A point on the ground shows up in two overlapping photos, and its image sits in a slightly different place in each. That shift is the parallax. Points closer to the camera, like a rooftop or a treetop, shift more than points on the ground. Measure the shift and you can work out the height.

The catch is that a short air base makes every shift small. If the two camera positions are nearly on top of each other, a tall building and a flat field shift by almost the same amount, and a tiny measuring error becomes a large height error. A long air base spreads the shifts apart, so the same measuring error costs less height.

Classic aerial survey flew about 60% end lap: enough for every point to appear in two photos, with a long base between them. Drone mapping software often asks for much more. Pix4D's general guidance, for example, is at least 75% front overlap, and more over forest. That is not a mistake. Modern software matches each point across many photos, and more photos mean more chances to match textured or tricky ground. But each neighboring pair now has a shorter base, so the precision you get from any one pair drops.

## How it is worked out

Start with the ground one photo covers along the flight line. It comes from the frame's length along track, the focal length, and the height above ground, the same similar triangles as [ground sample distance](/learn/ground-sample-distance/):

- **Coverage along track** = frame length × height / focal length

End lap is the share of that coverage the next photo repeats, so the air base is what is left:

- **Air base** = coverage × (1 − end lap)
- **Base-to-height ratio** = air base / height

Now the height precision. The parallax equation from the Army Corps of Engineers manual says a point's height is h = H − f B / p, where H is the flying height, f the focal length, B the air base, and p the parallax. Work out how much h changes when p changes by a small error σ_p and you get:

- **Height precision** = (height / air base) × (height / focal length) × σ_p

Read that left to right. The first factor is the inverse of the base-to-height ratio. Halve the ratio and the height error doubles. The second factor is the photo scale number. The last is how well you can measure parallax on the image, often a fraction of a pixel.

This is the geometric precision of one parallax measurement in one pair. It is not the accuracy of a finished map. The ASPRS accuracy standard measures product accuracy with surveyed checkpoints, not by predicting it from geometry, and software that combines many photos usually beats a single pair.

## A worked example

A common 1-inch drone camera has a 13.2 mm × 8.8 mm sensor, 5,472 pixels across, and an 8.8 mm lens. Fly it 100 m above flat ground with the short side of the sensor along the flight line, as the Penn State course suggests. Each photo covers 100 m along track, and each pixel covers about 2.74 cm.

Say you can measure parallax to half a pixel, about 0.0012 mm on the sensor. Here is what end lap does to one stereo pair.

**60% end lap**

- Air base: 40 m
- Base-to-height ratio: 0.40
- Height precision of one measurement: 3.4 cm

**75% end lap**

- Air base: 25 m
- Base-to-height ratio: 0.25
- Height precision: 5.5 cm

**80% end lap**

- Air base: 20 m
- Base-to-height ratio: 0.20
- Height precision: 6.9 cm

**90% end lap**

- Air base: 10 m
- Base-to-height ratio: 0.10
- Height precision: 13.7 cm

Going from 60% to 80% halves the base and doubles the height error of each pair. At 90% the photos are only 10 m apart from 100 m up, and a half-pixel slip moves a height by almost 14 cm.

So why fly 80%? Because each ground point then appears in about five photos along the line instead of two or three, and the software intersects all of those rays, not just one pair. The farthest photos that still see a point have a long base between them, which recovers much of the lost geometry. The tradeoff is real, though: more photos, more flight time, more processing, and pairs that are individually weaker. When heights matter most, keep enough overlap for reliable matching, but do not assume more overlap always means better heights.

The [air base tool](/drone/photogrammetry/stereo-base/) works these numbers for your camera, and the [parallax height tool](/drone/photogrammetry/parallax-height/) turns a measured parallax into a height.
