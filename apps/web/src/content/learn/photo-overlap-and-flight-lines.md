---
title: Photo overlap and flight lines explained
description: How front and side overlap set a mapping drone's trigger distance and line spacing, how many photos a job takes, and why overlap drops over high ground.
summary: Front and side overlap, trigger distance, line spacing, the image count, and overlap loss over hills.
audience: Drone operators
published: 2026-09-23
tools:
  - drone.photogrammetry.trigger
  - drone.photogrammetry.image-count
  - drone.photogrammetry.terrain-overlap
  - drone.mission.survey-grid
  - drone.photogrammetry.gsd
sources:
  - title: "GEOG 892: Geospatial Applications of Unmanned Aerial Systems, Lesson 4, Designing a Flight Route"
    issuer: Abdullah, Q., Penn State College of Earth and Mineral Sciences
    edition: Online course text, retrieved 2026-09-23
    locator: Flight lines computations; number of image computations
    url: https://courses.ems.psu.edu/geog892/node/658
  - title: Relationship of Topographic Relief, Flight Height, and Minimum and Maximum Overlap
    issuer: Pryor, W. T., Bureau of Public Roads, Highway Research Board Bulletin 228
    edition: 1959
    locator: Endlap and sidelap principles; the 50 percent stereo minimum
    url: https://onlinepubs.trb.org/Onlinepubs/hrbbulletin/228/228-005.pdf
  - title: Selecting the Image Acquisition Plan Type (PIX4Dmapper)
    issuer: Pix4D support documentation
    edition: Vendor guidance, retrieved 2026-09-23
    locator: Overlap for the general case, dense vegetation, and flat agricultural fields
    url: https://support.pix4d.com/hc/en-us/articles/202557459
---

Photo overlap is how much of each photo also appears in the next one. Front overlap (also called forward overlap or endlap) is the shared part along a flight line. Side overlap (sidelap) is the shared part between neighboring lines. Mapping software needs every spot on the ground in several photos to match them and build a 3D model, so overlap is what makes a set of photos into a map.

Two numbers turn overlap into a flight plan: the trigger distance, how far the drone flies between photos, and the line spacing, how far apart the flight lines are.

## Why it matters

Too little overlap leaves holes in the map or a model that will not process. Too much costs flight time, batteries, storage, and processing hours. The right amount depends on the ground: Pix4D's guidance for its own software asks for at least 75% front and 60% side overlap in the general case, at least 85% both ways over forest and dense vegetation, and at least 80% both ways over flat agricultural fields. These are one vendor's recommendations, not a standard, so follow your own software's guidance.

In the older aerial survey literature, Pryor (1959) notes that stereo coverage needs an absolute minimum of 50% endlap, and that in practice more is needed to tie one stereo model to the next.

## How it is worked out

Start with the photo's footprint, the ground one image covers. It comes from the camera and the height, the same way as [ground sample distance](/learn/ground-sample-distance/). Then:

- **Trigger distance** = along-track footprint × (1 − front overlap)
- **Line spacing** = across-track footprint × (1 − side overlap)
- **Trigger interval** = trigger distance / groundspeed

Penn State's GEOG 892 course then counts lines and photos for a rectangular area:

- **Lines** = width / line spacing + 1, rounded up
- **Photos per line** = length / photo spacing + 1, rounded up, plus two extra photos past each end of the line for continuous stereo coverage
- **Total photos** = lines × photos per line

The course also advises flying lines along the long side of the area, so there are fewer lines and fewer turns, and turning the camera so the long side of the sensor is across the flight direction.

## A worked example

A 1-inch camera (13.2 mm × 8.8 mm sensor, 5,472 pixels across, 8.8 mm lens) at 100 m above flat ground, 10 m/s, with 75% front and 65% side overlap:

| Step | Result |
|---|---|
| Footprint along track | 100 m |
| Footprint across track | 150 m |
| Trigger distance | **25 m** |
| Trigger interval | 2.5 s |
| Line spacing | **52.5 m** |

Now fly it over a field about 602 m long and 150 m wide (9.02 ha):

| Step | Result |
|---|---|
| Lines | 4 |
| Photos | **120** |
| Survey lines, end to end | 2.41 km |
| Path, with the extra photos and turns | 2.95 km |

That is 150 / 52.5 + 1 = 3.9, rounded up to 4 lines, and 602 / 25 + 1 = 25.1, rounded up to 26 photos, plus 4 extras, or 30 photos per line. The [survey grid tool](/drone/mission/survey-grid/) lays out the same plan and estimates about 4.9 minutes of flying at 10 m/s.

Raising the overlap to 80% front and 70% side shortens the trigger distance to 20 m and the line spacing to 45 m. The same field then takes 5 lines and 180 photos: 50% more photos for 5 points more overlap each way.

## Overlap over high ground

Many mapping apps hold a constant height above the takeoff point. The trigger distance and line spacing are fixed from that height, but where the ground rises the camera is closer to it and each photo covers less. Overlap drops. The overlap over a hill is:

overlap there = 1 − (1 − planned overlap) × height / (height − hill height)

Fly the example plan at 100 m above takeoff over a hill 40 m high:

| | Over flat ground | Over the hill |
|---|---|---|
| Height above the ground | 100 m | 60 m |
| Front overlap | 75% | **58.3%** |
| Side overlap | 65% | **41.7%** |

Side overlap drops the most, because it started lower. Two fixes:

- **Fly higher.** To keep at least 55% everywhere, the [terrain overlap tool](/drone/photogrammetry/terrain-overlap/) says to fly at 180 m above takeoff, which is above the Part 107 limit of 400 ft (about 122 m) above ground at the takeoff point.
- **Plan more overlap.** At 80% front and 70% side from 100 m, the overlap over the hill is 66.7% front and 50% side.

Terrain-following flight, where the drone holds a height above the ground instead of above takeoff, avoids the problem if your software supports it.

## Common mistakes

- **Planning from the takeoff height over hilly ground.** Check the overlap at the highest ground, not the average.
- **Swapping the sensor sides.** The long side of the sensor goes across the flight line. Swapping them shortens the line spacing and adds lines.
- **Forgetting the extra photos at each end.** The edges of the area then have less coverage than the middle.
- **Counting lines without the +1.** Four gaps need five lines.
- **Flying too fast for the camera.** Higher overlap, a lower height, or a faster groundspeed all shorten the trigger interval, and it can get shorter than the camera can take and save a photo. The trigger tool checks the interval against a minimum you enter.
- **Relying on wind-free groundspeed.** A tailwind shortens the time between photos at the same trigger distance.

## Where the numbers come from

The line and photo counts follow Penn State's GEOG 892 course, lesson 4. The 50% stereo minimum and the effect of relief on overlap come from Pryor's 1959 Highway Research Board bulletin. The overlap levels are Pix4D's published guidance. The [overlap and trigger tool](/drone/photogrammetry/trigger/) turns overlap into trigger distance and line spacing, the [image count tool](/drone/photogrammetry/image-count/) counts lines and photos for any area, including no-fly holes, and the [terrain overlap tool](/drone/photogrammetry/terrain-overlap/) finds the overlap over the highest ground and the height that keeps it. Each shows its steps under "How we got this."
