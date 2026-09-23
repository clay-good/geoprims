---
title: State plane coordinates explained
description: How SPCS 83 state plane zones work, Lambert vs. transverse Mercator zones, finding your zone, and the US survey foot vs. international foot.
summary: The flat grids US surveyors work in, how to find the right zone, and the 2 ppm foot that moves coordinates by feet.
audience: Surveyors
published: 2026-09-23
tools:
  - geodesy.spcs.spcs83-forward
  - geodesy.spcs.spcs83-inverse
  - geodesy.spcs.zone-lookup
  - geodesy.datum.nad83
  - geodesy.utm.forward
sources:
  - title: State Plane Coordinate System of 1983, NOAA Manual NOS NGS 5
    issuer: Stem, J. E., National Geodetic Survey
    edition: January 1989, reprinted with minor corrections March 1990
    locator: Chapter 1, projections used, the 1:10,000 design limit, and SPCS 83 zone changes
    url: https://geodesy.noaa.gov/library/pdfs/NOAA_Manual_NOS_NGS_0005.pdf
  - title: "U.S. Survey Foot: Frequently Asked Questions"
    issuer: National Institute of Standards and Technology
    edition: Web page
    locator: The 2 ppm difference, the December 31, 2022 deprecation, and SPCS 83 coordinates in US survey feet
    url: https://www.nist.gov/pml/us-surveyfoot/frequently-asked-questions-faqs
  - title: NGS and NIST to Retire U.S. Survey Foot after 2022
    issuer: National Geodetic Survey, NOAA
    edition: News release
    locator: The foot defined as 0.3048 meter exactly since 1959
    url: https://geodesy.noaa.gov/web/news/us-survey-foot.shtml
---

The State Plane Coordinate System of 1983 (SPCS 83) divides the United States into zones, each with its own flat grid on NAD 83. A position in a zone is an easting and a northing in meters or feet, like **1,347,294.025 ft E, 413,222.374 ft N** in Pennsylvania South. The zones are small enough that grid distances stay close to ground distances, which is why surveyors, engineers, and county GIS offices use them.

## Why it matters

Deeds, plats, road plans, and parcel maps in the US are often tied to state plane coordinates. A state plane grid is flat, so bearings and distances can be computed with plane trigonometry, the way a surveyor works in the field. UTM zones are 6° wide and can distort distance by as much as 1 part in 1,000 (see [UTM coordinates](/learn/utm-coordinates/)). State plane zones are drawn to keep that much smaller.

## How the zones are designed

The original state plane systems of the 1930s were designed to keep distortion under 1 part in 10,000, the accuracy of a steel tape at the time, according to NGS Manual NOS NGS 5. States that could not meet that with one zone got several. Most states kept the same zones for SPCS 83.

Each zone uses one of three conformal projections, chosen for the zone's shape:

| Projection | Used for | Examples |
|---|---|---|
| Lambert conformal conic | Zones long east to west | Colorado North, Central, and South; Pennsylvania North and South |
| Transverse Mercator | Zones long north to south | New York East, Central, and West; Florida East and West |
| Oblique Mercator | A zone lying on a slant | Alaska zone 1, the panhandle |

A state can mix types. Florida North is Lambert, while Florida East and West are transverse Mercator. New York Long Island is Lambert, and the rest of New York is transverse Mercator. Montana, Nebraska, and South Carolina chose a single zone for SPCS 83, and NGS notes that there the scale correction can exceed 1 part in 10,000.

## Finding your zone

In most states, zone boundaries follow county lines. Alaska's do not. The [zone lookup tool](/geodesy/spcs/zone-lookup/) searches by name or by point. For downtown Pittsburgh it returns two zones whose area boxes cover the point: Pennsylvania South (3702) and West Virginia North (4701). The tool then tells you to confirm the zone by county. Allegheny County is in Pennsylvania South.

Each zone has a four-digit NGS code, like 3702, and an EPSG code for GIS software. Pennsylvania South is EPSG 32129.

## A worked example

Pittsburgh at 40.446111° N, 79.982222° W (NAD 83) in Pennsylvania South, zone 3702:

| Result | Value |
|---|---|
| Easting | 1,347,294.025 US survey ft |
| Northing | 413,222.374 US survey ft |
| Same point in meters | 410,656.04 m E, 125,950.432 m N |
| Scale factor | 0.9999595 |
| Convergence | −1.44825° |

The scale factor means a grid distance here is about 40 parts per million shorter than the same distance on the ellipsoid, or about 0.04 ft in 1,000 ft. The convergence is the angle between grid north and true north at the point.

Compare UTM at the same spot. The [UTM forward tool](/geodesy/utm/forward/) puts Pittsburgh in zone 17N with a scale factor of 0.999691696, about 308 parts per million short, or 0.31 ft in 1,000 ft. Here the state plane grid distorts distance almost eight times less, which is the point of the smaller zones.

## US survey foot vs. international foot

The US has had two feet:

| Unit | Definition | Status |
|---|---|---|
| International foot | 0.3048 m exactly | The US foot since 1959 |
| US survey foot | 1200/3937 m | Deprecated by NIST and NOAA on December 31, 2022 |

They differ by 2 parts per million, about 0.01 ft in a mile. That sounds small, but state plane coordinates are big numbers. NIST notes that a coordinate of 1,000,000 ft changes by 2 ft if you swap feet. For the Pittsburgh point:

| Unit | Easting | Northing |
|---|---|---|
| US survey feet | 1,347,294.025 | 413,222.374 |
| International feet | 1,347,296.719 | 413,223.201 |
| Difference | 2.694 ft | 0.827 ft |

Reading the US survey foot numbers as international feet in the [inverse tool](/geodesy/spcs/spcs83-inverse/) plots the point 2.818 ft away, to the west-southwest.

NIST says NGS will keep providing SPCS 83 coordinates in US survey feet where a state defined them that way, as legacy data. The tools use each zone's legal foot by default. Montana, for example, is in international feet, and Colorado and Pennsylvania are in US survey feet. NIST also says NGS's new SPCS2022 uses the international foot only.

## Common mistakes

- **Guessing the foot.** Check the zone's legal unit and the metadata. A wrong foot moves coordinates by feet, not hundredths.
- **Using the wrong zone near a boundary.** Confirm by county, not by a map box.
- **Feeding in WGS 84 positions.** State plane coordinates are only as good as the NAD 83 positions behind them. A WGS 84 position fed in as NAD 83 carries the meter-level datum difference (see [NAD83 vs. WGS 84](/learn/nad83-vs-wgs84/)).
- **Treating grid distance as ground distance.** Apply the scale factor, and on high ground the elevation factor too.
- **Using SPCS 27 values.** Zones and constants changed in 1983. Old NAD 27 coordinates need a datum conversion first.

## Where the numbers come from

The zone definitions and projection equations are in NGS Manual NOS NGS 5. The foot definitions and the deprecation come from NIST and NGS. Start with the [zone lookup tool](/geodesy/spcs/zone-lookup/), then convert with the [state plane forward tool](/geodesy/spcs/spcs83-forward/), which shows each step under "How we got this."
