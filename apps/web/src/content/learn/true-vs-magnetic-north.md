---
title: True north vs. magnetic north
description: Magnetic declination, or variation, is the angle between true and magnetic north. How to find it for any place and date, and convert a bearing either way.
summary: What separates true, magnetic, and grid north, how the World Magnetic Model gives the angle between them, and how to convert a course without getting the sign backward.
audience: Pilots
published: 2026-09-23
tools:
  - geodesy.magnetic.declination
  - geodesy.magnetic.true-to-magnetic
  - geodesy.magnetic.grivation
sources:
  - title: The World Magnetic Model
    issuer: NOAA National Centers for Environmental Information
    edition: WMM2025, released December 17, 2024
    locator: Overview, update cycle, and secular variation
    url: https://www.ncei.noaa.gov/products/world-magnetic-model
  - title: Geomagnetism Frequently Asked Questions
    issuer: NOAA National Centers for Environmental Information
    edition: Web page, current
    locator: Declination and its sign, true bearing from a magnetic bearing
    url: https://www.ncei.noaa.gov/products/geomagnetism-frequently-asked-questions
  - title: International Geomagnetic Reference Field
    issuer: NOAA National Centers for Environmental Information, for IAGA
    edition: IGRF-14, finalized November 2024
    locator: Overview and model versions
    url: https://www.ncei.noaa.gov/products/international-geomagnetic-reference-field
  - title: The Universal Grids and the Transverse Mercator and Polar Stereographic Map Projections (NGA.SIG.0012)
    issuer: National Geospatial-Intelligence Agency
    edition: Version 2.0.0, 2014-03-25
    locator: Section 6.2, convergence of meridians (grid declination)
    url: https://earth-info.nga.mil/php/download.php?file=coord-utmups
---

True north is the direction along a meridian to the geographic North Pole. Magnetic north is the direction a compass needle points, along the horizontal part of Earth's magnetic field. The angle between them is magnetic declination, which pilots call variation. It is east when magnetic north lies east of true north and west when it lies west, and it changes with place and, slowly, with time.

## Why it matters

Charts, METARs, TAFs, and winds aloft give directions in degrees true. Your compass, heading indicator, runway numbers, and the wind from the tower are magnetic. Every time a direction crosses from one world to the other, you add or subtract the variation. In Boulder, Colorado, it is 7.7° E, and at Boston's Logan airport it is 14° W. Getting the sign wrong doubles the error instead of removing it.

## How it is worked out

Earth's main field is described by a model: a set of coefficients that give the field's direction and strength anywhere, at any date the model covers. The standard one for navigation is the World Magnetic Model (WMM), made by the US National Geospatial-Intelligence Agency and the UK Defence Geographic Centre, with NOAA and the British Geological Survey. It is also built into Android and iOS phones. A new version comes out every five years, and the current one, WMM2025, was released in December 2024.

The field drifts, so the model carries a rate of change for each coefficient. This slow drift is called secular variation. For dates back to 1900, the International Geomagnetic Reference Field (IGRF), from the International Association of Geomagnetism and Aeronomy, does the same job. Its 14th generation was finalized in November 2024.

From the model at your place and date, the [magnetic declination tool](/geodesy/magnetic/declination/) finds the north and east parts of the horizontal field. Declination is the angle between them: positive (east) when the field points east of true north.

## A worked example

Boulder, Colorado, at 1,655 m, on September 18, 2026, with WMM2025:

| Quantity | Result |
|---|---|
| Declination (variation) | **7.7° E** (7.67°) |
| Change per year | 0.081° toward the west |
| Model uncertainty | about 0.37° |
| Inclination (dip) | 66.02° |

A true course of 090° there becomes 082.3° magnetic, from the [true and magnetic bearing tool](/geodesy/magnetic/true-to-magnetic/) using the model value.

The same place over time, from IGRF-14:

| Date | Declination at Boulder |
|---|---|
| 1950 | 14.1° E |
| 2000 | 10.6° E |
| 2026 | 7.7° E |

In 76 years, magnetic north as seen from Boulder has swung more than 6° closer to true north. A chart printed a few years ago can be off by a few tenths of a degree, and a very old map by several degrees.

## East is least, west is best

The memory aid is for going from true to magnetic. Subtract an easterly variation. Add a westerly one.

| True course | Variation | Magnetic course |
|---|---|---|
| 090° | 7° E | 083° |
| 090° | 12° W | 102° |

To go the other way, from magnetic to true, reverse it: add east, subtract west. NOAA puts it the same way from the other side: add the declination to a magnetic bearing to get a true bearing, with east counted positive. With 12° W, a magnetic 102° gives a true 090°.

## Grid north and grivation

A third north comes from the map itself. On a UTM or UPS grid, grid north is the direction of the grid's vertical lines. The angle from true north to grid north is the convergence of meridians, which the NGA also calls grid declination. It is small near a UTM zone's center line and grows toward the zone edges and the poles.

Grid variation, or grivation, is the angle from grid north to magnetic north: declination minus convergence. It is what you apply when you navigate by a map grid with a compass. In Boulder, which is in UTM zone 13N, convergence is −0.1739°, and the [grivation tool](/geodesy/magnetic/grivation/) gives 7.8° E, barely different from the declination. Near the pole it is another story. At 86° N, 45° E on the polar grid (UPS north), declination is 45.32° and convergence 45°, so grivation is only 0.3° E. Declination alone would be off by 45°.

Near the magnetic poles the compass itself becomes unreliable. At that polar point the horizontal field is 3,230 nT, under the 6,000 nT limit the WMM uses to flag a caution zone, and the tool warns that compass readings may be degraded.

## Common mistakes

- **Applying the variation backward.** True to magnetic: east is least, west is best.
- **Mixing true and magnetic winds.** METAR, TAF, and winds aloft winds are true. The tower and ATIS give magnetic. See [how to read a METAR](/learn/how-to-read-a-metar/) and [how to read a winds aloft forecast](/learn/winds-aloft-forecast/).
- **Using an old variation.** The value on an old chart or map drifts every year.
- **Confusing variation with deviation.** Deviation is the error of your own compass, caused by the aircraft, and is on the compass correction card. It is applied after variation.
- **Ignoring convergence on a grid map.** On a UTM map, use grivation, not declination.

## Where the numbers come from

The models are WMM2025 and IGRF-14, both published through NOAA's National Centers for Environmental Information. The declination sign convention follows NOAA, and the convergence definition follows the NGA's standard for the universal grids. The [magnetic declination tool](/geodesy/magnetic/declination/) gives declination, its yearly change, and the model's uncertainty for any place and date. The [true and magnetic bearing tool](/geodesy/magnetic/true-to-magnetic/) converts a bearing with a chart variation, the model, or both side by side. For navigation, use the variation printed on your current chart.
