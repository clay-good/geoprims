## Why

The time domain computes the sun to 0.0003° but knows nothing of the moon or the stars, and the navigation domain has no sight reduction, which `add-practitioner-essentials` listed as "possible later." The gaps are felt in three places:
- Pilots and drone operators planning night work ask when the moon rises and how bright it is; military and search teams ask the same of moon illumination.
- Navigators, students, and the celestial-navigation community reduce sights by hand from almanac tables and want a check: Hc, Zn, and the intercept from a sextant altitude, plus the almanac data itself, without buying tables that are partly Crown copyright.
- Surveyors still take solar and Polaris observations for an astronomic azimuth; the hour-angle method needs the sun's or Polaris's GHA and declination at the instant.

All of it is one astronomy engine: positions of the sun, moon, planets, and stars at an instant, sidereal time, and the spherical triangle. The sources are open (Meeus's formulas, VSOP87 and ELP-2000/82 series, IAU 2006/2000A precession-nutation via ERFA, FK5 star positions, USNO definitions), the oracle is the USNO Astronomical Applications API, and the worked examples are in Bowditch chapter 19.

Depends on: `add-practitioner-essentials` (NREL SPA, ΔT, DUT1, tzdb, leap seconds), `add-marine-navigation` (Bowditch sources, sailing diagrams), `add-geodesy-suite` (magnetic variation for the compass-check tools).

## What Changes

| Group | Tools | What they answer |
|---|---|---|
| `navigation.celestial` | almanac, sight-reduction, altitude-correction, polaris-latitude, sun-azimuth-check | GHA and Dec of the sun, moon, Venus, Mars, Jupiter, Saturn, Aries, and the 57 navigational stars plus Polaris at an instant; Hc, Z, Zn, and intercept from an assumed position; sextant to observed altitude (dip, refraction, semidiameter, parallax); latitude by Polaris; the sun's true azimuth and amplitude for a compass check |
| `time.moon` | position, rise-set, phase | Moon azimuth and altitude; moonrise, moonset, and transit by USNO's definition; phase, illuminated fraction, age, and the next four principal phases |
| `time.scale` (extended) | sidereal, unix-time, iso-week | GMST, GAST, LMST, LAST and the equation of the equinoxes; Unix seconds ↔ UTC; ISO week dates |
| `survey.astro` | solar-azimuth, polaris-azimuth | Astronomic azimuth of a line from a timed solar or Polaris observation by the hour-angle method, with the semidiameter correction and the geodetic-astronomic note |

## Capabilities

### New Capabilities

- `navigation/celestial`: Computed almanac data, sight reduction, altitude corrections, Polaris latitude, and the sun azimuth check.
- `time/moon`: Lunar position, rise and set, and phases.
- `time/sidereal-and-calendar`: Sidereal times, Unix time, and ISO week dates.
- `survey/astronomic-azimuth`: Solar and Polaris azimuth reduction for surveyors.

### Modified Capabilities

None.

## Non-goals

- Reproducing the Nautical Almanac, Air Almanac, Pub. 229, or their tables. The tools compute; they do not print tables.
- Lunar distance clearing, star identification from an unknown body, or planning which stars to shoot (possible later).
- Ephemeris precision beyond the Nautical Almanac's 0.1' (no JPL ephemeris file; the series are built in).

## Impact

- A new `gp-astro` crate shared by `gp-navigation`, `gp-time`, and `gp-survey` (statically linked per the module rules), holding VSOP87 (abridged), ELP-2000/82 (abridged), IAU 2006/2000A precession-nutation, the FK5 navigational-star table, and the astronomical triangle. Module budgets are checked; the series are the size risk.
- New sources-ledger rows: Meeus, *Astronomical Algorithms*, 2nd ed. (1998); VSOP87 and ELP-2000/82 (VizieR VI/81, VI/79); IERS Conventions 2010 ch. 5; Capitaine, Wallace & Chapront 2005; ERFA (reference); FK5 (VizieR I/149); Bennett 1982 (refraction); USNO rise/set definitions and API (oracle); the Nautical Almanac (cited for conventions only); Bowditch chapters 15 and 19; POSIX.1-2024 §4.19; ISO 8601-1:2019.
- New codes: `UT1_APPROXIMATED` already exists; add `BODY_BELOW_HORIZON` (info) and `POLAR_MOON` (info, no rise or set that day).
