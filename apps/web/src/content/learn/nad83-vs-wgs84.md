---
title: NAD83 vs. WGS 84 explained
description: Why NAD 83(2011) and WGS 84 coordinates for the same spot differ by more than a meter, why the gap grows each year, and how to convert.
summary: Datums, realizations, and epochs, and why a meter separates two systems people treat as the same.
audience: Surveyors
published: 2026-09-23
tools:
  - geodesy.datum.nad83
  - geodesy.datum.plate-motion
  - geodesy.datum.nadcon5
  - geodesy.datum.itrf
  - geodesy.datum.legacy
sources:
  - title: HTDP Revision Log
    issuer: National Geodetic Survey, NOAA
    edition: HTDP 3.6.0, April 7, 2025
    locator: Version 3.6.0, WGS 84 (G2296) added, equal to ITRF2020 and used by NGA from January 7, 2024
    url: https://geodesy.noaa.gov/TOOLS/Htdp/HTDP-log.pdf
  - title: The National Adjustment of 2011, NOAA Technical Report NOS NGS 65
    issuer: Dennis, M. L., National Geodetic Survey
    edition: July 29, 2020
    locator: Executive summary, the three NAD 83 frames and the 2010.00 epoch
    url: https://geodesy.noaa.gov/library/pdfs/NOAA_TR_NOS_NGS_0065.pdf
  - title: "NADCON 5.0: Geometric Transformation Tool for points in the National Spatial Reference System, NOAA Technical Report NOS NGS 63"
    issuer: Smith, D., and Bilich, A., National Geodetic Survey
    edition: July 24, 2017, revised August 31, 2020
    locator: NAD 27 to NAD 83(1986) grids and later NAD 83 realizations
    url: https://geodesy.noaa.gov/library/pdfs/NOAA_TR_NOS_NGS_0063.pdf
  - title: WGS 84
    issuer: National Geospatial-Intelligence Agency, Office of Geomatics
    edition: Web page
    locator: Alignment of the WGS 84 reference frame with the ITRF
    url: https://earth-info.nga.mil/index.php?dir=wgs84&action=wgs84
---

NAD 83 and WGS 84 are two different geodetic datums. They started out almost the same in the 1980s, but today they differ by more than a meter across the United States, and the gap grows every year. A latitude and longitude means nothing precise until you know which datum, and which version of it, it is on.

The short reason: WGS 84 is tied to the whole Earth, and NAD 83 is tied to the North American plate. The plate moves, so the two slowly pull apart.

## What a datum is, and what a realization is

A datum says where the coordinate grid sits on the Earth: where its center is, how it is turned, and what ellipsoid it uses. NAD 83 and WGS 84 use nearly the same ellipsoid, so the difference between them is not about shape. It is about where the frame is anchored.

Each datum has been updated several times. Each update is a **realization**, and it has its own tag:

| Datum | Current realization | Anchored to |
|---|---|---|
| WGS 84 | G2296, used by NGA since January 7, 2024 | The whole Earth; aligned with ITRF2020 |
| NAD 83 | NAD 83(2011), epoch 2010.00 | The North American plate |

NGS also keeps NAD 83(PA11) for the Pacific plate and NAD 83(MA11) for the Mariana plate. The HTDP revision log from the National Geodetic Survey (NGS) treats WGS 84 (G2296) as equal to ITRF2020, the international frame. NGA says the WGS 84 frame is aligned with the ITRF to within a centimeter.

## Why the gap keeps growing

A coordinate in a plate-fixed frame like NAD 83(2011) stays put while the ground under it moves with the plate. A coordinate in an Earth-fixed frame like WGS 84 or ITRF2020 changes as the plate drifts. That is why an ITRF or WGS 84 coordinate needs an **epoch**: the date it was true.

The [plate motion tool](/geodesy/datum/plate-motion/) shows the drift. With the ITRF2020 plate model, a point in central Kansas moves 14.57 mm a year west and 3.41 mm a year south. From 2010.0 to 2026.7 that adds up to 0.2498 m.

The same drift shows up in the datum difference. At that Kansas point the gap between WGS 84 (G2296) and NAD 83(2011) was 1.161 m at epoch 2010.0 and is 1.3573 m at epoch 2026.7.

## How it is worked out

NGS publishes the relationship as a 14-parameter Helmert transformation in its HTDP software: three shifts, three rotations, a scale change, and a yearly rate for each. The steps are:

1. Turn latitude, longitude, and ellipsoidal height into Earth-centered X, Y, Z.
2. Work out the seven parameters at your epoch from their rates.
3. Rotate, scale, and shift X, Y, Z into the other frame.
4. Turn the result back into latitude, longitude, and height.

The [NAD 83 transformation tool](/geodesy/datum/nad83/) follows HTDP's route through ITRF94 and reports the difference as east, north, and up.

## A worked example

A point in central Kansas at 38.5° N, 98° W, 500 m ellipsoidal height, given in WGS 84 (G2296) at epoch 2026.7 and moved to NAD 83(2011):

| Result | Value |
|---|---|
| East | 1.2039 m |
| North | −0.6267 m |
| Horizontal difference | **1.3573 m**, toward 117.5° |
| Up | 1.0234 m |
| NAD 83(2011) height | 501.0234 m |

The same point on NAD 83(2011) plots about 1.36 m east-southeast of its WGS 84 spot, and its ellipsoidal height is about 1 m larger. The size and direction change across the country. At epoch 2026.7 the horizontal difference is 1.5763 m in San Francisco and 1.2588 m in New York.

## Old datums: NAD 27 and NADCON5

NAD 27 is a much older datum, built outward from Meades Ranch in Kansas. Its errors vary from place to place, so a single set of parameters cannot convert it. NGS publishes grids of shifts instead, called NADCON 5.0. At Meades Ranch the [NADCON5 tool](/geodesy/datum/nadcon5/) moves a NAD 27 position 29.815 m, toward 272.1°, to reach NAD 83(1986).

Outside North America, older datums such as ED50 are often converted with published Helmert parameters. The [legacy datum tool](/geodesy/datum/legacy/) moves a point in Paris 138.7 m from ED50 to WGS 84, and warns that this transformation is good only to about 10 m.

## Rules of thumb

- **NAD 83(2011) and WGS 84 differ by about 1 to 2 m in the conterminous United States.** For a phone map or a hiking GPS that is noise. For a boundary or a construction stakeout it is not.
- **The gap grows by the plate's drift.** In the Kansas example that is about 1.5 cm a year, or about 0.2 m from 2010 to 2026.7.
- **ITRF realizations are close to each other.** The [ITRF tool](/geodesy/datum/itrf/) moves a Pittsburgh point only 0.0027 m from ITRF2020 to ITRF2014 at epoch 2026.72. The big jump is between a plate-fixed frame and an Earth-fixed one.

## Common mistakes

- **Treating "WGS 84" and "NAD 83" as the same.** Some software does, with a zero transformation. That silently puts a meter or more of error in the data.
- **Leaving out the epoch.** A WGS 84 coordinate from 2010 and one from 2026 for the same mark differ by the plate drift.
- **Using the wrong realization.** NAD 83(2011) for a point on the Pacific plate, such as Hawaii, is off by meters. Use PA11 there.
- **Mixing heights.** Datum changes move ellipsoidal heights too. Convert the height before applying a geoid model (see [geoid vs. ellipsoid height](/learn/geoid-vs-ellipsoid/)).

## Where the numbers come from

The transformation parameters are those in NGS's HTDP 3.6.0. The NAD 83 frames and their 2010.00 epoch are described in NGS Technical Report 65, and the NAD 27 grids in NGS Technical Report 63. Each tool shows its steps under "How we got this." Start with the [NAD 83 transformation tool](/geodesy/datum/nad83/) for modern GNSS data, and the [NADCON5 tool](/geodesy/datum/nadcon5/) for anything on NAD 27.
