---
title: Grid vs ground distance explained
description: Why a state plane distance is shorter than the one you taped, and how the grid scale factor, elevation factor, and combined factor convert between them.
summary: Why state plane distances differ from tape distances, and the one factor that converts between them.
audience: Surveyors
published: 2026-09-23
tools:
  - survey.reduction.combined-factor
  - geodesy.spcs.spcs83-forward
  - geodesy.spcs.zone-lookup
  - survey.cogo.area-by-coordinates
sources:
  - title: State Plane Coordinate System of 1983 (NOAA Manual NOS NGS 5)
    issuer: National Geodetic Survey (Stem, J. E.)
    edition: 1989, reprinted with corrections 1990
    locator: Section 2.6, grid scale factor at a point; Section 4.1, elevation factor and combined factor
    url: https://geodesy.noaa.gov/library/pdfs/NOAA_Manual_NOS_NGS_0005.pdf
  - title: "The State Plane Coordinate System: History, Policy, and Future Directions (NOAA Special Publication NOS NGS 13)"
    issuer: National Geodetic Survey (Dennis, M. L.)
    edition: Version 1, March 6, 2018
    locator: Item C4, scaling of SPCS coordinates to ground; the Montana example of linear distortion at the ground surface
    url: https://geodesy.noaa.gov/library/pdfs/NOAA_SP_NOS_NGS_0013_v01_2018-03-06.pdf
---

A ground distance is the horizontal distance you measure on the earth's surface with a tape or a total station. A grid distance is the same line measured between state plane (or UTM) coordinates. The two differ because the grid is a flat map of a curved ellipsoid, and your line sits some height above that ellipsoid. One number, the combined factor, converts between them: grid distance = ground distance × combined factor.

The grid distance is usually a little shorter than the ground distance. In the worked example below, at 1,500 ft above the ellipsoid, it is 0.162 ft shorter in 1,000 ft, about 1 part in 6,200. That is small, but a total station can measure a 1,000 ft line much more closely than that, so the difference shows up in closures, stakeout checks, and areas.

## Why it matters

Surveyors mix the two kinds of distance all the time. Control comes from GNSS as state plane coordinates. Field crews measure ground distances. A plat or a set of construction plans may show either. If a crew stakes a grid distance with a tape, every point is off by the scale difference. If you inverse between state plane coordinates and write that distance on a plat as if it were measured on the ground, the next surveyor who measures it will find a "bust" that is not a mistake at all.

NGS notes that the grid distance is almost always different from, and usually shorter than, the horizontal ground distance. In high country the gap can be much larger than the nominal 1:10,000 that state plane zones were designed around. Its example is Missoula, Montana, where 100 m on the ground is about 7.6 cm shorter on the grid.

## How it is worked out

The conversion has two parts, and each is a factor close to 1.

1. **Grid scale factor (k).** A map projection cannot keep every distance true. The grid scale factor says how much the projection stretches or shrinks a short line at a point. It is exactly 1 on the projection's standard lines, less than 1 between them, and more than 1 outside them. It depends only on where you are in the zone. The [state plane tool](/geodesy/spcs/spcs83-forward/) reports it with the coordinates; for Pittsburgh in the Pennsylvania South zone it is 0.9999595.
2. **Elevation factor.** Your line is above the ellipsoid, so it is slightly longer than its shadow on the ellipsoid. The elevation factor is R / (R + h), where R is a mean earth radius (NGS uses 20,906,000 ft) and h is the ellipsoid height. Ellipsoid height is the orthometric elevation plus the geoid height: h = H + N.

The **combined factor** is the product of the two. Multiply a ground distance by it to get a grid distance. Divide a grid distance by it to get a ground distance.

## A worked example

A line measured at 1,000 ft on the ground, at an ellipsoid height of 1,500 ft, where the grid scale factor is 0.99991:

| Step | Result |
|---|---|
| Grid scale factor (given) | 0.99991 |
| Elevation factor, R / (R + h) | 0.99992825 |
| Combined factor | **0.99983826** |
| Ground distance | 1,000 ft |
| Grid distance | **999.838 ft** |

The grid distance is 0.162 ft shorter. Going the other way, 1,000 ft between state plane coordinates is 1,000.162 ft on the ground. Over a mile the difference grows to almost 0.9 ft: 5,280 ft on the ground is 5,279.146 ft on the grid.

## Rules of thumb, and how far they drift

**Add the factors instead of multiplying them.** NGS Manual 5 notes that the product of two factors near 1 is close to their sum minus 1. Here, 0.99991 + 0.99992825 − 1 = 0.99983825, against the exact 0.99983826. The shortcut is off by about a hundredth of a part per million, which is far below anything you can measure.

**About 48 ppm for every 1,000 ft of height.** Because R is about 20.9 million ft, the elevation factor shrinks by about 1 part per million for every 21 ft of ellipsoid height. At 6,000 ft of ellipsoid height, with the same grid scale factor, the combined factor drops to 0.9996231, and 1,000 ft on the ground is 999.623 ft on the grid.

**Leaving out the geoid height.** NGS says skipping the geoid height changes reduced distances by 0.16 ppm for each meter of geoid height. In the example, using the elevation of 1,530 ft as if it were the ellipsoid height (so ignoring a geoid height of −30 ft) gives a combined factor of 0.99983683 instead of 0.99983826, or 999.837 ft instead of 999.838 ft. That is small on one line, but it is a bias, so it never averages out.

## Common mistakes

- **Using sea-level elevation instead of ellipsoid height.** In the conterminous United States the geoid height is negative, so the ellipsoid height is lower than the elevation. Use h = H + N.
- **Using a factor from the wrong place.** The grid scale factor changes across a zone, and the elevation factor changes with height. One project-wide factor is fine for a small, flat site, but a long route or a site with a lot of relief may need a factor per line.
- **Scaling coordinates and still calling them state plane.** Dividing state plane coordinates by a combined factor gives "ground" coordinates. NGS points out that these are no longer state plane coordinates, and that every project tends to pick its own factor. If you scale, record the factor, the origin, and the zone on the plat.
- **Forgetting areas.** A grid area converts to a ground area by dividing by the combined factor squared, not by the factor once. The [area by coordinates tool](/survey/cogo/area-by-coordinates/) works on the grid, so apply the factor to its result.
- **Mixing feet.** State plane coordinates may be in meters, US survey feet, or international feet. NGS Manual 5 notes the two feet differ by 2 parts per million, so a mix-up adds a scale error of its own on top of the combined factor.

## Where the numbers come from

The elevation factor, the 20,906,000 ft mean radius, and the combined factor are defined in NGS Manual NOS NGS 5 by James Stem. NGS Special Publication 13 explains why grid and ground differ in practice and the problems with scaling coordinates to ground. The [combined factor tool](/survey/reduction/combined-factor/) does the conversion both ways and shows each factor under "How we got this." To find the zone for a point, start with the [state plane zone lookup](/geodesy/spcs/zone-lookup/), then take the grid scale factor from the [state plane conversion](/geodesy/spcs/spcs83-forward/).
