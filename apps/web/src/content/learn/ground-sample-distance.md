---
title: What is ground sample distance (GSD)?
description: GSD is how much ground one image pixel covers. How to work it out from sensor width, focal length, image width, and height, and what it does and does not say about accuracy.
summary: How much ground one pixel covers, how to work it out, the height for a target GSD, and why GSD is not accuracy.
audience: Drone operators
published: 2026-09-23
tools:
  - drone.photogrammetry.gsd
  - drone.photogrammetry.altitude-for-gsd
  - drone.photogrammetry.asprs-accuracy
  - drone.photogrammetry.trigger
  - drone.ops.part107-altitude
sources:
  - title: "GEOG 892: Geospatial Applications of Unmanned Aerial Systems, Lesson 4, Geometry of Vertical Image"
    issuer: Abdullah, Q., Penn State College of Earth and Mineral Sciences
    edition: Online course text, retrieved 2026-09-23
    locator: Scale of vertical image; scale from a digital camera and GSD
    url: https://courses.ems.psu.edu/geog892/node/657
  - title: "GEOG 892, Lesson 4, Designing a Flight Route"
    issuer: Abdullah, Q., Penn State College of Earth and Mineral Sciences
    edition: Online course text, retrieved 2026-09-23
    locator: Flight altitude computations (H = f × GSD / pixel size)
    url: https://courses.ems.psu.edu/geog892/node/658
  - title: "GEOG 892, Lesson 8, The New ASPRS Positional Accuracy Standards for Digital Geospatial Data"
    issuer: Abdullah, Q., Penn State College of Earth and Mineral Sciences
    edition: Online course text on ASPRS Edition 2, Version 2 (2024), retrieved 2026-09-23
    locator: Motivation and highlights (accuracy thresholds independent of GSD; RMSE only; checkpoint survey accuracy included)
    url: https://courses.ems.psu.edu/geog892/node/707
  - title: "GEOG 892, Lesson 8, The New ASPRS Standards and Number of Check Points"
    issuer: Abdullah, Q., Penn State College of Earth and Mineral Sciences
    edition: Online course text on ASPRS Edition 2, Version 2 (2024), retrieved 2026-09-23
    locator: Minimum of 30 checkpoints; checkpoints by project area
    url: https://courses.ems.psu.edu/geog892/node/710
  - title: 14 CFR 107.51, Operating limitations for small unmanned aircraft
    issuer: Federal Aviation Administration, eCFR
    edition: Current as of 2026-09-01
    locator: §107.51(b), 400 feet above ground level
    url: https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-107/subpart-B/section-107.51
---

Ground sample distance (GSD), also called ground sampling distance, is the width of ground that one pixel of an aerial image covers. A GSD of 2.7 cm means each pixel spans 2.7 cm on the ground, so the smallest thing you can pick out is a few pixels, or several centimeters, across. GSD depends on the camera and on how high above the ground you fly: fly twice as high and each pixel covers twice as much ground.

GSD is the first number in almost every mapping flight plan. It sets the flying height, which sets the photo footprint, which sets the overlap spacing and the number of photos.

## Why it matters

A client asks for "2 cm imagery," or a job needs enough detail to see cracks, paint lines, or survey targets. GSD is how you turn that request into a height. It also drives the cost of the job: halving the GSD halves the footprint in each direction, so it takes about four times as many photos to cover the same area.

In the United States, Part 107 limits a small drone to 400 ft above ground level, unless it stays within a 400 ft radius of a structure and no higher than 400 ft above the structure's top (14 CFR 107.51(b)). That caps the coarsest GSD a given camera can reach under the rule, and it is the first thing to check when the height for a target GSD comes out high.

## How it is worked out

A vertical photo is a pair of similar triangles that meet at the lens. The sensor lies one focal length behind the lens, and the ground lies one flying height below it. Penn State's GEOG 892 course writes the scale as focal length over flying height. For a digital camera the useful form is per pixel:

GSD = sensor width × height above ground / (focal length × image width in pixels)

Sensor width divided by image width is the pixel pitch, the physical size of one pixel on the chip. Turn the formula around and you get the height for a target GSD:

Height = target GSD × focal length × image width / sensor width

Three details matter:

- **Height above ground,** not above takeoff or sea level. Over rising ground the GSD gets finer, and the overlap drops.
- **Physical focal length,** not the "35 mm equivalent" printed in many drone specifications.
- **Matching units.** Keep sensor width and focal length in the same unit, and the GSD comes out in the unit of the height.

## A worked example

A drone with a 1-inch, 20 megapixel camera: a 13.2 mm × 8.8 mm sensor, 5,472 × 3,648 pixels, and an 8.8 mm lens, flying 100 m above the ground:

| Step | Result |
|---|---|
| GSD | **2.741 cm** |
| Footprint across track | 150 m |
| Footprint along track | 100 m |

Change the height and the GSD scales with it:

| Height above ground | GSD | Footprint across track |
|---|---|---|
| 72.96 m | 2 cm (target) | 109.4 m |
| 100 m | 2.741 cm | 150 m |
| 120 m | 3.289 cm | 180 m |
| 400 ft (121.9 m) | 3.342 cm | 182.9 m |

For a 2 cm GSD this camera flies at 72.96 m. For 1 inch per pixel, the [height for a GSD tool](/drone/photogrammetry/altitude-for-gsd/) gives 92.66 m.

## What GSD says about accuracy, and what it does not

GSD is resolution, not accuracy. A sharp image can still sit in the wrong place on the earth if the camera calibration, the GNSS, or the ground control is poor, and a map made from coarser pixels can be more accurate than one made from finer pixels.

Older map standards tied accuracy to scale or pixel size. ASPRS Edition 2 of its *Positional Accuracy Standards for Digital Geospatial Data* (Version 2, 2024) does not. Penn State's summary of it says that accuracy measures based on map scale, film scale, and GSD no longer apply, and that the new accuracy classes are independent of GSD. Accuracy is stated as root mean square error (RMSE) measured at independent checkpoints, with a minimum of 30 checkpoints, and the accuracy of the checkpoint survey itself is counted in. With a 1.00 cm vertical fit and checkpoints surveyed to 2.0 cm, the [ASPRS accuracy tool](/drone/photogrammetry/asprs-accuracy/) reports 2.24 cm RMSE.

So choose GSD for what you need to see, and prove accuracy with checkpoints.

## Common mistakes

- **Using the 35 mm equivalent focal length.** Entering 24 mm as if it were the physical focal length of this camera gives 1.005 cm, almost three times too fine. The GSD tool flags a focal length that looks too long for the sensor. Entered as a 35 mm equivalent, 24 mm becomes 8.8 mm and the GSD is 2.741 cm again.
- **Measuring height from the takeoff point.** If the ground rises 40 m under a 100 m flight, the camera is 60 m above it there. The GSD is finer, but the overlap drops, and that can break the map. See [photo overlap and flight lines](/learn/photo-overlap-and-flight-lines/).
- **Mixing up sensor width and height.** Use the long side of the sensor with the long side of the image in pixels.
- **Treating GSD as a spec for accuracy.** Contract accuracy is RMSE at checkpoints, not a pixel size.
- **Ignoring motion blur.** At slow shutter speeds and fast groundspeeds, the drone moves more than a pixel while the shutter is open, and the real resolution is worse than the GSD. The [motion blur tool](/drone/photogrammetry/motion-blur/) finds the slowest safe shutter speed.

## Where the numbers come from

The image geometry and the height formula follow Penn State's GEOG 892 course, lesson 4, which draws on Wolf, Dewitt, and Wilkinson's *Elements of Photogrammetry*. The accuracy points come from Penn State's lesson 8 summary of the ASPRS Edition 2 standards, and the height limit from 14 CFR 107.51. The [GSD tool](/drone/photogrammetry/gsd/) shows each step under "How we got this." Once you have a height, the [overlap and trigger tool](/drone/photogrammetry/trigger/) turns it into photo spacing and line spacing, and the [Part 107 altitude tool](/drone/ops/part107-altitude/) checks it against the rule.
