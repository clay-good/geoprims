---
title: Geoid vs. ellipsoid height explained
description: Why a GPS height is not a sea-level height, how the geoid height N connects them (H = h − N), and when to use EGM96 or NGS GEOID18.
summary: Two kinds of height, the geoid height that links them, and the tens of meters you lose by mixing them up.
audience: Surveyors
published: 2026-09-23
tools:
  - geodesy.height.convert
  - geodesy.geoid.geoid-height
  - geodesy.datum.nad83
sources:
  - title: GEOID18 computation
    issuer: National Geodetic Survey, NOAA
    edition: GEOID18
    locator: Coverage (conterminous US, Puerto Rico, US Virgin Islands) and NAD 83 input positions
    url: https://geodesy.noaa.gov/GEOID/GEOID18/computation.html
  - title: The Development of the Joint NASA GSFC and NIMA Geopotential Model EGM96, NASA/TP-1998-206861
    issuer: Lemoine, F. G., et al., NASA Goddard Space Flight Center
    edition: 1998
    locator: Geoid undulations relative to WGS 84
    url: https://ntrs.nasa.gov/citations/19980218814
  - title: GeographicLib Geoid class and geoid data
    issuer: Karney, C. F. F., GeographicLib
    edition: GeographicLib 2.x, egm96-15 grid
    locator: The relation h = N + H, and interpolation errors for each grid
    url: https://geographiclib.sourceforge.io/C++/doc/geoid.html
---

A GPS receiver measures height above the ellipsoid, a smooth mathematical shape that fits the whole Earth. Surveyors, engineers, and maps use height above the geoid, which is close to mean sea level. The gap between the two is the geoid height, N, and it can be tens of meters, so the two kinds of height are never interchangeable.

The link is one line: **H = h − N**. Here h is the ellipsoidal height from GPS, H is the orthometric height (the "elevation" on a benchmark or a map), and N is how far the geoid sits above the ellipsoid at that spot.

## Why it matters

The ellipsoid is simple. It is flattened at the poles and has no bumps, which makes it easy to compute with. It also has no physical meaning: water does not run downhill along it.

The geoid is the surface that water would settle on if the oceans could flow freely under the continents. It rises and falls with the pull of gravity, so it is lumpy. "Uphill" and "downhill" follow the geoid, which is why drainage design, flood maps, and floor elevations all use orthometric height.

The two surfaces are far apart. With the EGM96 model used by the tools here:

| Place | Geoid height N |
|---|---|
| Central Kansas (38.5°, −98°) | −28.074 m |
| Denver | −16.993 m |
| New York (JFK) | −32.567 m |
| Timbuktu, Mali | 28.708 m |
| Indian Ocean south of India (5°, 78°) | −104.685 m |
| New Guinea (−5°, 145°) | 70.456 m |

A negative N means the geoid is below the ellipsoid. At the three US places above it is 17 to 33 m below, so a GPS height there reads lower than the elevation.

## How it is worked out

1. **Get the ellipsoidal height, h.** A GNSS receiver or a processing service reports it. Note which datum it is on, such as WGS 84 or NAD 83(2011), because the same point can have a different h in each (see [NAD83 vs. WGS 84](/learn/nad83-vs-wgs84/)).
2. **Look up the geoid height, N, at the point.** A geoid model is a grid of N values. The [geoid height tool](/geodesy/geoid/geoid-height/) reads the EGM96 15-minute grid and interpolates between grid points.
3. **Subtract.** H = h − N. To go the other way, h = H + N. The [height conversion tool](/geodesy/height/convert/) does both.

Interpolation matters. With the 12-point cubic method the EGM96 grid carries at most 0.169 m of interpolation error. With simple bilinear interpolation the worst case is 1.152 m. Those are the limits GeographicLib publishes for this grid, and the geoid height tool reports the one for the method you pick.

## A worked example

A GPS height of 100 m at Timbuktu, Mali:

| Step | Value |
|---|---|
| Ellipsoidal height h | 100 m |
| Geoid height N (EGM96) | 28.708 m |
| Orthometric height H = h − N | **71.292 m** |

Here the geoid is above the ellipsoid, so the elevation is less than the GPS height. In central Kansas the sign flips. A GPS height of 500 m there has N = −28.074 m, so H = 500 − (−28.074) = 528.074 m. The elevation is higher than the GPS height.

## EGM96 or GEOID18?

EGM96 is a global model from NASA and the agency now called NGA. It describes mean sea level worldwide to about a meter.

In the United States, official elevations are on NAVD 88, not on a global geoid. The National Geodetic Survey (NGS) publishes a hybrid geoid model, GEOID18, built to turn NAD 83(2011) ellipsoid heights into NAVD 88 heights. It covers the conterminous United States, Puerto Rico, and the US Virgin Islands. NGS says Alaska, Hawaii, Guam, and the Northern Mariana Islands should keep using GEOID12B.

The two models answer different questions:

| Model | Heights it gives | Use it for |
|---|---|---|
| EGM96 | Global mean sea level, on WGS 84 | Worldwide work and rough checks |
| GEOID18 | NAVD 88, from NAD 83(2011) | US survey and engineering elevations |

The tools on this site use EGM96 only. For a US benchmark elevation, use NGS's GEOID18 tools; EGM96 can differ from NAVD 88 by a meter or more, and the tool says so in a warning with every result.

## Common mistakes

- **Reading a GPS height as elevation.** In Kansas that puts you 28 m too low, and in New York 33 m.
- **Getting the sign backward.** N is the geoid above the ellipsoid. H = h − N; with N negative, H comes out larger than h.
- **Mixing datums.** GEOID18 expects NAD 83(2011) positions and heights. Feeding it a WGS 84 height adds the meter-level datum difference to your answer.
- **Using EGM96 for NAVD 88.** A global model is not the national vertical datum.
- **Assuming every "MSL" is the same.** A GPS unit or drone may compute its sea-level height from a global model like EGM96. Check its manual before comparing it with survey elevations.

## Where the numbers come from

The EGM96 model is described in NASA's 1998 technical paper (NASA/TP-1998-206861). The grid and its interpolation limits come from GeographicLib, and the US model and its coverage come from NGS. The [height conversion tool](/geodesy/height/convert/) shows each step for your point under "How we got this." To move a position between NAD 83 and WGS 84 before converting a height, use the [NAD 83 transformation tool](/geodesy/datum/nad83/).
