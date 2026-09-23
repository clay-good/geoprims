---
title: Haversine vs. geodesic distance explained
description: How far off the haversine formula is compared with the true geodesic on the WGS 84 ellipsoid, with real city pairs, plus rhumb lines and Vincenty.
summary: Great circles on a sphere, geodesics on the ellipsoid, and how many kilometers the shortcut costs.
audience: Developers and GIS
published: 2026-09-23
tools:
  - navigation.geodesic.haversine
  - navigation.geodesic.inverse
  - navigation.rhumb.inverse
  - navigation.geodesic.vincenty-inverse
  - navigation.geodesic.spherical-inverse
sources:
  - title: Algorithms for geodesics
    issuer: Karney, C. F. F., Journal of Geodesy 87(1), 43–55
    edition: 2013 (arXiv:1109.4448)
    locator: Solution of the inverse problem, including nearly antipodal points
    url: https://arxiv.org/abs/1109.4448
  - title: GeographicLib geodesic documentation and test data
    issuer: Karney, C. F. F., GeographicLib
    edition: GeographicLib 2.x
    locator: Geodesics on an ellipsoid of revolution; GeodTest.dat, 500,000 geodesics
    url: https://geographiclib.sourceforge.io/C++/doc/geodesic.html
  - title: Direct and inverse solutions of geodesics on the ellipsoid with application of nested equations
    issuer: Vincenty, T., Survey Review 23(176), 88–93
    edition: April 1975
    locator: Section 4, the inverse formula, and its note on nearly antipodal points
    url: https://geodesy.noaa.gov/PUBS_LIB/inverse.pdf
  - title: GeographicLib Rhumb class
    issuer: Karney, C. F. F., GeographicLib
    edition: GeographicLib 2.x
    locator: Rhumb lines on the ellipsoid, and how much longer they can be than the geodesic
    url: https://geographiclib.sourceforge.io/C++/doc/classGeographicLib_1_1Rhumb.html
---

The haversine formula gives the great-circle distance between two points on a sphere. The Earth is not a sphere. It is slightly flattened, and the true shortest path over its surface, the geodesic, is up to a few tenths of a percent longer or shorter than haversine says. For New York to London that is 14.890 km.

Whether that matters depends on the job. For sorting nearby stores it does not. For survey, aviation, legal boundaries, or any distance you report to the meter, use the geodesic.

## Why the two differ

Haversine assumes one radius everywhere. The tools here use 6,371,008.771 m, the mean radius R1 of the ellipsoid. The real Earth's radius is 6,378,137 m at the equator and 6,356,752.314 m at the poles, about 21 km less, so its curvature changes with latitude and direction.

- **North-south lines** at low latitudes run where the meridian curves more sharply than the sphere. A degree of latitude there is shorter, so haversine reads **long**.
- **East-west lines** near the equator run where the Earth curves more gently than the sphere. A degree there is longer, so haversine reads **short**.

That is why the sign of the error flips from one route to the next, and why no single "correction factor" fixes haversine.

## How each is worked out

**Haversine.** Take the two latitudes and the difference in longitude. The formula finds the central angle between the points, then multiplies by the radius: d = 2R · asin(√(sin²(Δφ/2) + cos φ₁ cos φ₂ sin²(Δλ/2))). It is short, fast, and easy to put in a spreadsheet.

**Geodesic.** The [geodesic distance tool](/navigation/geodesic/inverse/) solves the inverse problem on the WGS 84 ellipsoid with Charles Karney's 2013 algorithm, the one in GeographicLib. It maps the ellipsoid onto an auxiliary sphere, solves there, and corrects with series in the ellipsoid's flattening. It converges for every pair of points, including nearly opposite ones.

## Worked examples

From the [haversine tool](/navigation/geodesic/haversine/), which reports both distances:

| Route | Haversine | Geodesic (WGS 84) | Haversine error |
|---|---|---|---|
| New York JFK to London Heathrow | 5,540.019 km | 5,554.909 km | 14,890 m short (0.27%) |
| Los Angeles to San Francisco | 543.663 km | 543.534 km | 129 m long (0.02%) |
| Fairbanks to Utqiaġvik | 808.498 km | 811.137 km | 2,639 m short (0.33%) |
| Helsinki to Cape Town | 10,500.595 km | 10,466.179 km | 34,416 m long (0.33%) |
| Sydney to Los Angeles | 12,061.039 km | 12,050.608 km | 10,431 m long (0.09%) |
| Equator, 0° to 45° N along a meridian | 5,003.779 km | 4,984.944 km | 18,834 m long (0.38%) |

In these examples the error ranges from 0.02% to 0.38%. On a 500 km flight that is up to about 2 km. On a transoceanic route it is tens of kilometers.

The spherical formula also gets the starting course slightly wrong. For JFK to Heathrow, the [spherical great-circle tool](/navigation/geodesic/spherical-inverse/) starts on 51.3525209°, while the geodesic starts on 51.3816479°, a difference of 0.0291°.

## Rhumb lines

A rhumb line (loxodrome) keeps one compass course all the way. It is easy to steer but longer than the geodesic. The [rhumb line tool](/navigation/rhumb/inverse/) compares the two:

| Route | Rhumb line | Constant course | Extra over the geodesic |
|---|---|---|---|
| JFK to Heathrow | 5,774.19 km | 77.9684139° | 219.281 km (3.95%) |
| Helsinki to Cape Town | 10,466.845 km | 183.2593481° | 0.666 km (0.01%) |
| Anchorage to Moscow | 10,036.718 km | 266.6905666° | 3,039.415 km (43.44%) |

A nearly north-south route barely changes. A high-latitude east-west route can be far longer, because the geodesic cuts over the pole while the rhumb line follows the parallels around.

## Vincenty's method and where it fails

Before Karney, many programs used Thaddeus Vincenty's 1975 iterative method. It is accurate: for JFK to Heathrow the [Vincenty tool](/navigation/geodesic/vincenty-inverse/) is 0.0118 mm from Karney. But Vincenty's own paper warns that the inverse "may give no solution" between nearly antipodal points. From 0°, 0° to 0.5° N, 179.7° E, the tool reports that Vincenty did not converge in 200 iterations. Karney's method returns 19,944.127 km for the same pair.

If your code uses Vincenty, it needs a fallback for those cases. Or switch to the Karney algorithm, which is in GeographicLib and many GIS libraries.

## Rules of thumb

- **Haversine is off by tenths of a percent.** In the examples above it is never off by more than 0.38%. Use it for rough ranking and quick displays.
- **Don't correct it with a factor.** The error changes sign with direction.
- **Use the geodesic for anything you report or store.**

## Common mistakes

- **Using a different radius.** 6,371 km, 6,378.137 km, and 6,371.0088 km all appear in code, and each shifts the answer. With the equatorial radius, JFK to Heathrow comes out 5,546.217 km instead of 5,540.019 km. Neither matches the geodesic, 5,554.909 km.
- **Mixing up degrees and radians.** The formula needs radians.
- **Assuming the great circle is the course to steer.** The geodesic's course changes along the way; only the rhumb line holds one course.
- **Trusting Vincenty on every pair.** It can fail near antipodes. Check that your code handles the failure.

## Where the numbers come from

Karney's algorithm is published in the *Journal of Geodesy* (2013) and implemented in GeographicLib, whose test set has 500,000 geodesics. Vincenty's method is in *Survey Review* (1975). Every tool shows its steps under "How we got this." Start with the [geodesic distance tool](/navigation/geodesic/inverse/) for a distance you will use, and the [haversine tool](/navigation/geodesic/haversine/) to see what a spherical shortcut costs.
