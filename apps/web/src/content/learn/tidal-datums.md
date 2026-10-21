---
title: Tidal datums
description: What MLLW, MSL, MHHW, and the other tidal datums mean, why a depth on a chart and a height on a survey can disagree by meters, and how to move a height from one datum to another with a station's datum sheet.
summary: Chart datum, mean sea level, and NAVD88 at one station, and the subtraction that connects them.
audience: Mariners
published: 2026-10-20
tools:
  - navigation.tide.datums
  - navigation.tide.predict
  - geodesy.height.convert
sources:
  - title: Tidal Datums and Their Applications, NOAA Special Publication NOS CO-OPS 1
    issuer: NOAA National Ocean Service, Center for Operational Oceanographic Products and Services
    edition: June 2000
    locator: Chapter 6, first-reduction tidal datums, and the glossary (mean lower low water)
    url: https://tidesandcurrents.noaa.gov/publications/tidal_datums_and_their_applications.pdf
  - title: Tidal datums, NOAA CO-OPS
    issuer: NOAA Center for Operational Oceanographic Products and Services
    edition: Datum options page and station datum sheets, read 2026-10-20
    locator: Datum definitions; the Boston, MA (8443970) datum sheet, epoch 1983 to 2001
    url: https://tidesandcurrents.noaa.gov/datum_options.html
---

A depth of 10 feet on a US chart does not mean 10 feet below the sea as it is right now. It means 10 feet below mean lower low water, a level that the tide reaches or passes on most days. At Boston that level sits more than five feet below the land survey's zero.

Every height near the coast is measured from some datum, and the coast has a lot of them. They are all fixed to one station, and its datum sheet tells you how far apart they are.

## What a tidal datum is

A tidal datum is an average of one phase of the tide, taken over a long, fixed period. NOAA's definition of the one on US charts is plain: mean lower low water (MLLW) is "the average of the lower low water height of each tidal day observed over the National Tidal Datum Epoch."

The epoch is a 19-year span. Nineteen years is long enough to cover the 18.6-year cycle of the Moon's orbit, which slowly widens and narrows the tides. Boston's datums use the 1983 to 2001 epoch. (NOAA's 2000 handbook names 1960 to 1978 as current; the newer epoch replaced it after the handbook was written.)

The common ones, from the top of a station's sheet down:

- **MHHW**, mean higher high water: the average of each day's higher high tide.
- **MHW**, mean high water: the average of all high tides.
- **MSL**, mean sea level: the average of the hourly heights.
- **MTL**, mean tide level: halfway between MHW and MLW.
- **MLW**, mean low water: the average of all low tides.
- **MLLW**, mean lower low water: the average of each day's lower low tide, and US chart datum.

A sheet also lists **STND**, the station's own zero, and often **NAVD88**, the land survey datum, measured at the same tide gauge. That shared gauge is what links the water to the land.

## Why it matters

Datums are local. NOAA's handbook warns that they "should not be extended into areas which have differing hydrographic characteristics without substantiating measurements." The tide's range changes from harbor to harbor, so the gap between MLLW and NAVD88 changes too.

Mixing them is an easy mistake. A surveyor's flood elevation is on NAVD88; a mariner's depth is on MLLW; a tide table's heights are on MLLW; a sea level report may be on MSL. Put two of those in one sum without converting and the answer is off by the distance between datums, which can be several feet.

Some sheet rows are not datums at all. GT, the great diurnal range, is the distance from MLLW to MHHW. DHQ and DLQ are small inequalities between the tides. HWI and LWI are time intervals in hours. They are useful numbers, but you cannot measure a height from them.

## How it is worked out

Every datum on the sheet is given as a height above the station datum. That makes conversion one line:

- **Height above the new datum** = height above the old datum + (old datum − new datum)

Nothing else is involved. No tide, no time, no geoid. The sheet holds the whole answer, for that station and that epoch.

## A worked example

Boston's sheet (station 8443970) gives, in meters above station datum:

- MHHW 4.205
- MSL 2.660
- NAVD88 2.752
- MLLW 1.074

A survey needs a mark that sits 3.000 m above MLLW on NAVD88.

- Old datum, MLLW: 1.074
- New datum, NAVD88: 2.752
- Difference: 1.074 − 2.752 = −1.678
- **Height on NAVD88: 3.000 − 1.678 = 1.322 m**

Run it the other way to check. NAVD88 sits 1.678 m above MLLW at Boston, so a point at 0 on NAVD88 is 1.678 m above chart datum. That is about 5.5 feet, the gap mentioned at the top of this page.

Two limits are worth knowing. The sheet is for one station only; between stations, NOAA's VDatum models how the gaps change along the coast. And datums move when NOAA adopts a new epoch, because the sea itself has moved. Note which epoch a height was measured in.

The [tidal datum tool](/navigation/tide/datums/) does this subtraction from a sheet you paste and draws the stack to scale. The [tide prediction tool](/navigation/tide/predict/) gives tide heights on MLLW and can hand one straight to it, and the [height converter](/geodesy/height/convert/) handles ellipsoid and geoid heights on land.
