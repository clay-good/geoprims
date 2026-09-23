---
title: MGRS and USNG coordinates explained
description: How to read an MGRS or USNG grid reference, what the grid zone and 100 km square letters mean, and how many digits you need for 10 m or 1 m.
summary: Grid zone, 100 km square, and digits, and why a shorter reference is a bigger square, not a rounder number.
audience: Developers and GIS
published: 2026-09-23
tools:
  - geodesy.grid-ref.mgrs-forward
  - geodesy.grid-ref.mgrs-inverse
  - geodesy.grid-ref.usng-forward
  - geodesy.grid-ref.usng-inverse
  - geodesy.utm.forward
sources:
  - title: Universal Grids and Grid Reference Systems, NGA.STND.0037
    issuer: National Geospatial-Intelligence Agency
    edition: Version 2.0.0, February 28, 2014
    locator: Chapter 3, the Military Grid Reference System (3-2 grid zone designation, 3-3 100,000-meter square, 3-4 truncation)
    url: https://earth-info.nga.mil/php/download.php?file=coord-grids
  - title: United States National Grid, FGDC-STD-011-2001
    issuer: Federal Geographic Data Committee
    edition: December 2001
    locator: Relationship to datums (NAD 83 or WGS 84), precision, and Annex B, truncation of USNG values
    url: https://www.fgdc.gov/standards/projects/usng/fgdc_std_011_2001_usng.pdf
---

An MGRS reference like **17T NE 86309 77770** names a square on the ground. The first part, 17T, is the grid zone. The two letters, NE, name a 100 km square inside it. The digits are an easting and a northing inside that square, split in half: 86309 east and 77770 north, here to the nearest meter. USNG, the US National Grid, is the same system written with spaces.

MGRS stands for Military Grid Reference System. It is built on UTM (see [UTM coordinates](/learn/utm-coordinates/)), but it swaps the long millions-of-meters numbers for two letters, so a reference is short enough to read aloud or jot on a map.

## Why it matters

NGA publishes MGRS for the US Department of Defense. USNG is the civilian version, a federal standard from the Federal Geographic Data Committee (FGDC). Both let you say where something is to 10 m in 13 characters, like 17TNE86307777, and both work with a paper map and a ruler, because the grid lines are printed on the map.

## How to read it

An MGRS reference has three parts, set out in the NGA standard:

| Part | Example | What it tells you |
|---|---|---|
| Grid zone designation | 17T | UTM zone 17, latitude band T (40° N to 48° N) |
| 100 km square | NE | Which 100,000 m square in that zone |
| Digits | 86309 77770 | Easting, then northing, within the square |

The **grid zone designation** is the UTM zone number plus a latitude band letter. Bands run from C at 80° S to X at 84° N, skipping I and O. Each is 8° tall, except X, which is 12°.

The **100 km square** letters repeat in a pattern across zones. The first letter steps east through the zone and the second steps north. They are unique only inside one grid zone, so "NE 86309 77770" is ambiguous unless everyone knows the zone. Locally, the zone is often left off.

The **digits** always come in an even count. The first half is the easting, the second half the northing, both measured from the square's southwest corner. They are the last digits of the UTM easting and northing. Pittsburgh's UTM easting is 586,309.953 m and its northing 4,477,770.428 m. Drop everything above the 100 km place and the meter fractions, and you get 86309 and 77770.

## Reading it off a paper map

The FGDC standard gives the map reader's rule: **read right, then up**. Find the vertical grid line just left of the point and read its large digits. Measure how far right the point is from that line. Then find the horizontal line just below the point, read its digits, and measure up. On a map with a 1,000 m grid, two digits for each line plus one estimated digit for each distance give a 6-digit reference, good to 100 m. Write the easting half first, then the northing half.

## Precision by digits

Fewer digits make a bigger square, not a rounder number. The NGA standard says to shorten a reference by **truncation**, never rounding. A reference names the southwest corner of its square. From the [MGRS encoding tool](/geodesy/grid-ref/mgrs-forward/) for Pittsburgh (40.446111° N, 79.982222° W):

| Digits | Reference | Square size |
|---|---|---|
| 0 | 17T NE | 100,000 m |
| 2 | 17T NE 8 7 | 10,000 m |
| 4 | 17T NE 86 77 | 1,000 m |
| 6 | 17T NE 863 777 | 100 m |
| 8 | 17T NE 8630 7777 | 10 m |
| 10 | **17T NE 86309 77770** | 1 m |

Look at the 10 m line. The full easting is 86309, and truncating gives 8630, not 8631. The point is somewhere inside the 10 m square whose southwest corner is at 8630.

Pick the digits to match what you know. If your position is good to about 10 m, an 8-digit reference says so honestly, and a 10-digit one claims more than you have.

## A worked example, backward

To read a reference back, the [MGRS decoding tool](/geodesy/grid-ref/mgrs-inverse/) gives the square's corner and center. For **17TNE8630977770**, a 1 m square, the center is 40.4461117°, −79.9822273°.

A shorter reference is a bigger square. For **17TNE8677**, a 1,000 m square:

| Point | Latitude | Longitude |
|---|---|---|
| Southwest corner | 40.4392033° | −79.9859808° |
| Center | 40.4436553° | −79.9800181° |

Read a 4-digit reference as "somewhere in this kilometer," and plot the center if you need one point.

## USNG

The US National Grid, in FGDC standard FGDC-STD-011-2001, uses the same zones, squares, and digits as MGRS. The standard names NAD 83, or its international equivalent WGS 84, as the datum. It writes references with spaces, **17T NE 86309 77770**, and it allows a short local form such as NE 863 777 when the grid zone is known. The [USNG decoding tool](/geodesy/grid-ref/usng-inverse/) takes that short form with a zone of 17T and returns a 100 m square centered at 40.445922835°, −79.981752606°.

## Common mistakes

- **Rounding instead of truncating.** It can name the next square east or north, one the point is not in.
- **Uneven digits.** 17TNE863777 is a 100 m square. 17TNE86377 is invalid: five digits cannot split into an easting and a northing, and the decoding tool refuses it.
- **Dropping the grid zone across a boundary.** The same letters and digits repeat in other zones.
- **Reading a reference as a point.** It is a square. Its size is set by the number of digits.
- **Treating the band letter as N or S.** In MGRS, "S" is a latitude band north of the equator, not the southern hemisphere.

## Where the numbers come from

The rules are in the NGA grids standard, NGA.STND.0037, chapter 3, and the FGDC USNG standard. Each tool shows its steps under "How we got this." To encode a point, start with the [MGRS encoding tool](/geodesy/grid-ref/mgrs-forward/). For a USNG reference with spaces, use the [USNG encoding tool](/geodesy/grid-ref/usng-forward/).
