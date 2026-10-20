---
title: RVR and visibility
description: What runway visual range measures, how it differs from the visibility in a weather report, and how to turn an RVR minimum into statute miles when RVR is not reported.
summary: RVR versus prevailing visibility, and the 91.175(h) table that converts one to the other.
audience: Pilots
published: 2026-10-20
tools:
  - aviation.ifr.rvr-visibility
  - aviation.ifr.approach-category
  - aviation.weather.metar-decode
sources:
  - title: 14 CFR 91.175, Takeoff and landing under IFR
    issuer: FAA (eCFR)
    edition: Current eCFR text (amended 2018-04-27)
    locator: Paragraph (h), comparable values of RVR and ground visibility, and the (h)(2) table
    url: https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-91/subpart-B/section-91.175
  - title: Instrument Procedures Handbook (FAA-H-8083-16B)
    issuer: Federal Aviation Administration
    edition: FAA-H-8083-16B
    locator: Chapter 1, Runway Visual Range (RVR) and its types
    url: https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/instrument_procedures_handbook
---

An approach chart may give its visibility minimum as RVR 2400. If the runway's RVR is not being reported, that number becomes 1/2 statute mile. The conversion is a seven-row table in 14 CFR 91.175(h), and it is not interpolated.

The rest of this page explains why there are two measures of visibility at all, and how to read the table without inventing rows it does not have.

## Two ways to measure how far you can see

**Prevailing visibility** is what a weather report gives you. An observer, or an automated sensor, judges how far you can see around the airport. A METAR reports it in statute miles, like `1/2SM`. It describes the airport as a whole.

**Runway visual range (RVR)** is narrower and more precise. The Instrument Procedures Handbook defines it as an instrument-derived value for how far a pilot sees down the runway from the approach end. A transmissometer near the touchdown zone measures how clear the air is, and the system works out how far the runway lights or other targets would be visible. It is reported in hundreds of feet, like `R16/2400FT`.

Two details in that definition matter:

- RVR is **horizontal**, not slant. It is what you see rolling down the runway, not what you see from the approach path.
- RVR is **per runway**. A big airport can have one runway in thick fog and another in the clear.

When a runway has RVR and it is reported, it is the number that counts for that runway's minimums. The handbook says RVR is used in place of prevailing visibility for a particular runway.

## When RVR is not reported

Sometimes the chart's minimum is an RVR value but nobody is reporting RVR for your runway. The sensor may be out of service, or the runway may not have one.

For that case, 91.175(h)(1) says the RVR minimum is converted to ground visibility using the table in (h)(2), and that visibility becomes the minimum. This applies to takeoff and landing minimums, except Category II and III minimums.

Here is the whole table:

- **RVR 1,600 ft**: 1/4 statute mile
- **RVR 2,400 ft**: 1/2 statute mile
- **RVR 3,200 ft**: 5/8 statute mile
- **RVR 4,000 ft**: 3/4 statute mile
- **RVR 4,500 ft**: 7/8 statute mile
- **RVR 5,000 ft**: 1 statute mile
- **RVR 6,000 ft**: 1 1/4 statute miles

## Reading the table honestly

Notice what the table is not. It is not a unit conversion. 2,400 feet is not half a mile; half a mile is 2,640 feet. The rows pair minimums that the FAA treats as comparable, and the gaps between them are uneven: 1,600 to 2,400 ft is a quarter mile, but 4,000 to 4,500 ft is an eighth.

That is why a calculator should not draw a line between rows. An RVR of 2,800 feet does not have a regulation value. If an approach printed an RVR the table lacks, the honest answer is that the table does not cover it, not a number made up halfway between two rows.

## A worked example

You are planning an ILS whose chart lists the minimum as RVR 2400. The METAR shows no RVR group for your runway, and the NOTAMs say the touchdown sensor is out.

- Chart minimum: RVR 2,400 ft
- RVR reported: no
- Table row: 2,400 ft is 1/2 statute mile
- **Visibility minimum for this approach: 1/2 statute mile**

Now check the METAR's prevailing visibility against that. If it says `3/4SM`, you meet the converted minimum. If it says `1/4SM`, you do not.

Two cautions. First, the conversion does not apply to Category II or III minimums; those need RVR. Second, your operator may have rules stricter than Part 91. Commercial operators often do.

The [RVR to visibility tool](/aviation/ifr/rvr-visibility/) looks up any row of the table and refuses values the table does not hold. The [METAR decoder](/aviation/weather/metar-decode/) reads the visibility and any RVR groups out of a report, and the [approach category tool](/aviation/ifr/approach-category/) finds which column of minimums is yours.
