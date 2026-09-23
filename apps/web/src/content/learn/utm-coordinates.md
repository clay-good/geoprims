---
title: UTM coordinates explained
description: How UTM zones, eastings, and northings work, why the scale factor is 0.9996, the false easting and northing, and the Norway and Svalbard exceptions.
summary: The 60 zones, the 500,000 m false easting, the 0.9996 scale factor, and how to read a UTM coordinate.
audience: Developers and GIS
published: 2026-09-23
tools:
  - geodesy.utm.forward
  - geodesy.utm.inverse
  - geodesy.utm.zone
  - geodesy.grid-ref.mgrs-forward
  - geodesy.ups.forward
sources:
  - title: The Universal Grids and the Transverse Mercator and Polar Stereographic Map Projections, NGA.SIG.0012
    issuer: National Geospatial-Intelligence Agency
    edition: Version 2.0.0, March 25, 2014
    locator: Section 7.1, Definition of UTM, and Section 7.4, Administrative rules
    url: https://earth-info.nga.mil/php/download.php?file=coord-utmups
  - title: Universal Grids and Grid Reference Systems, NGA.STND.0037
    issuer: National Geospatial-Intelligence Agency
    edition: Version 2.0.0, February 28, 2014
    locator: Section 2-4 (UTM grid distortion) and Section 3-2 (zone 32 and zones 31 to 37 in the Arctic)
    url: https://earth-info.nga.mil/php/download.php?file=coord-grids
---

UTM (Universal Transverse Mercator) splits the Earth between 80° S and 84° N into 60 zones, each 6° of longitude wide. Inside a zone, a position is two distances in meters: an easting, measured east from a line 500,000 m west of the zone's center, and a northing, measured north from the equator. A full UTM coordinate is the zone, the hemisphere, the easting, and the northing, like **17N 586309.953 4477770.428**.

## Why it matters

Latitude and longitude are angles. A degree of longitude is 111.319 km at the equator and 55.799 km at 60° N, so you cannot measure distance or area with them directly. UTM turns the curved Earth into flat squares in meters, so distance is the Pythagorean theorem and area is length times width. That is why UTM is common in GIS layers, drone mapping, military maps, and field GPS units.

The price is that every zone is its own flat map. Coordinates from two zones do not line up, and distances are slightly stretched or shrunk depending on where you are in the zone.

## How it is worked out

Each zone is a transverse Mercator projection: a cylinder wrapped around the Earth, touching along the zone's central meridian. The NGA standard fixes the numbers for every zone:

| Parameter | Value |
|---|---|
| Zone width | 6° of longitude; zone 1 is centered on 177° W |
| Central meridian | −183° + 6° × zone number |
| Scale factor on the central meridian | 0.9996 exactly |
| False easting | 500,000 m |
| False northing | 0 m north of the equator, 10,000,000 m south of it |

The **false easting** keeps eastings positive. The central meridian is always 500,000 m, and eastings run from roughly 166,000 m to 834,000 m at the equator. The **false northing** does the same for the southern hemisphere: northings count down from 10,000,000 m at the equator.

The **scale factor of 0.9996** shrinks the map slightly on the central meridian so that it is too small in the middle and too large near the edges. The scale reaches exactly 1 about 180 km either side of the center: on the equator the tool gives 1.000002388 at an easting of 680,289.625 m. The NGA grids standard says that within a zone the difference from ground distance can be as much as 1 part in 1,000.

A point's zone number is set by its longitude, and a letter adds a latitude band (C to X, with no I or O), 8° tall except band X, which is 12°. The [UTM zone tool](/geodesy/utm/zone/) gives both.

## A worked example

Pittsburgh, at 40.446111° N, 79.982222° W:

| Result | Value |
|---|---|
| Zone | 17 N |
| Easting | 586,309.953 m |
| Northing | 4,477,770.428 m |
| Grid convergence | 0.6603064° |
| Scale factor | 0.999691696 |

Reading it: zone 17 is centered on 81° W. Pittsburgh's easting of 586,309.953 m puts it about 86 km east of that central meridian. Its northing says it is about 4,478 km north of the equator along the grid. The scale factor means 1,000 m on the ground is 999.692 m on the grid here. The convergence is the angle between grid north and true north.

For contrast, from the [UTM forward tool](/geodesy/utm/forward/):

| Point | Zone | Easting | Scale factor |
|---|---|---|---|
| 40.446111°, −81° (on the central meridian) | 17N | 500,000 m | 0.9996 |
| 0°, −78° (zone edge, on the equator) | 18N | 166,021.443 m | 1.000981062 |
| Sydney, −33.8688°, 151.2093° | 56S | 334,368.634 m | 0.999938201 |

Sydney's northing is 6,250,948.345 m. It is in the southern hemisphere, so that number is 10,000,000 m minus its distance south of the equator.

## Zone exceptions: Norway and Svalbard

Two areas break the 6° rule, under the NGA standard:

- **Southwest Norway.** Between 56° N and 64° N, zone 32 is widened to 9°, taking the western part from zone 31. A point at 60° N, 5° E would normally be in zone 31. The zone tool puts it in **32V**.
- **Svalbard.** Between 72° N and 84° N, zones 32, 34, and 36 are not used, and zones 31, 33, 35, and 37 are widened to cover the gaps. A point at 78° N, 10° E, normally zone 32, is in **33X**.

The polar caps, north of 84° N and south of 80° S, use a different grid, UPS. See the [UPS tool](/geodesy/ups/forward/).

## Common mistakes

- **Leaving out the zone or hemisphere.** The same easting and northing exist in all 60 zones, and in both hemispheres.
- **Mixing zones in one dataset.** Across a zone boundary, coordinates jump. For work that crosses a boundary, you can force one zone for the whole project. The tool allows a zone within 3 of the standard one. Forcing Pittsburgh into zone 18 gives an easting of 77,414.531 m and a scale factor of 1.001798885, with a warning that distortion is larger.
- **Treating grid distance as ground distance.** Divide a grid distance by the scale factor to get the distance on the ellipsoid, or use a [geodesic distance](/navigation/geodesic/inverse/) instead.
- **Forgetting the datum.** UTM on WGS 84 and UTM on NAD 83 differ by the datum difference (see [NAD83 vs. WGS 84](/learn/nad83-vs-wgs84/)).
- **Confusing the band letter with the hemisphere.** "17S" in the MGRS style is band S, which is in the northern hemisphere. Write N or S for the hemisphere, or give the full band.

## Where the numbers come from

The zone parameters and administrative rules are in NGA.SIG.0012, and the exceptions and latitude bands in NGA.STND.0037. The tools compute the projection with Karney's Krüger series. Try the [UTM forward tool](/geodesy/utm/forward/) and the [UTM inverse tool](/geodesy/utm/inverse/), which shows each step under "How we got this." For a grid reference built on UTM, see [MGRS and USNG coordinates](/learn/mgrs-coordinates/).
