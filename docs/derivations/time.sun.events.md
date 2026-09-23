<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Sunrise, sunset, and twilight (`time.sun.events`)

## Method

For the requested local date, find solar noon, then each time the sun's center crosses an altitude: −0.833° for sunrise and sunset (34′ of refraction plus the 16′ semidiameter, less the horizon dip when a height is given), and −6°, −12°, and −18° for civil, nautical, and astronomical twilight. The sun's geometric elevation comes from the NREL Solar Position Algorithm. On each side of noon, the day's highest and lowest points (found by golden-section search) decide whether the sun crosses at all, which gives the polar states and handles a twilight that grazes its altitude by hundredths of a degree. Bisection between those points then finds the time. Each side is reported on its own, so an evening twilight can end just before midnight after a night when it never began.

## Equations

- Elevation e(t) = 90° − topocentric zenith from SPA (Reda and Andreas 2008), without refraction, with ΔT from Espenak and Meeus.
- Crossing of altitude h on one side: e(t) − h changes sign between the lower culmination (noon ± 12 h) and the upper culmination (noon).
- Above: min e > h on that side. Below: max e < h.
- Solar noon: 12:00 − 4 × longitude − equation of time (NOAA), in minutes UT.

## Symbols and units

Latitude and longitude in degrees, the date as local YYYY-MM-DD, and the offset as ±HH:MM or an IANA zone. Times are rounded to the minute and shown in local time and Zulu, each dated. Day length is in minutes.

## Domain

Any latitude and longitude, and dates in SPA's range (−2000 to 6000). The polar states are polar-day and polar-night for sunrise and sunset, and no-…-twilight-begin, no-…-twilight-end, or no-…-twilight for the twilights. These codes stay in the result for machines; the page and the summary show them in plain words, like "None: the sun does not get low enough for civil twilight to begin".

## Approximations

Refraction at the horizon is the standard 34′. Real refraction varies with the weather by several arcminutes, which moves sunrise by a minute or more. UT1 is taken as UTC (under 1 s). Solar noon uses the NOAA equation of time (within a few seconds).

## Worked example

- sourcePublisher: United States Naval Observatory, Astronomical Applications Department
- sourceTitle: Sun and Moon Data for One Day (API rstt/oneday)
- sourceEdition: API 4.0.1, retrieved 2026-09-18
- sourceLocator: Denver (39.7392, −104.9903), 2026-06-21, UTC−6
- independent: yes
- inputs: 39.7392, −104.9903, date 2026-06-21, offset −06:00
- outputs: civil dawn 05:00, sunrise 05:32, solar noon 13:02, sunset 20:31, civil dusk 21:04 (local)
- tolerance: 1 minute (both round to the minute)
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-time/tests/usno_sun.rs`: 1,200 events (sunrise, sunset, civil dawn and dusk at 300 seeded places and dates, from 72°S to 72°N, including polar day and night) against the USNO API. All are within 1 minute, and 94% are to the same minute. A civil twilight that grazes −6° by 0.013° (67.19°S on February 1) matches USNO's lone 23:59 end.
- `tools/vectors/gen_usno_sun.py`: regenerates that fixture from the USNO API
- `core/vectors/time.sun.events.jsonl`: 24 vectors, including USNO day lengths, polar states, the grazing twilight, and the plain words a reader sees

## Invariants

- `core/crates/gp-time/tests/time.rs` `sun_events_invariants`: the nine events come in order, solar noon sits midway between sunrise and sunset, and in June the day lengthens with latitude from 60°S to 65°N
