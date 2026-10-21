## Context

Research gathered 2026-10-19. Facts that shape the design:
- The Nautical Almanac is joint USNO/HMNAO; the HMNAO material is Crown copyright, so tables must not be reproduced. The data can be computed from open series: VSOP87 (sun, planets), ELP-2000/82 (moon), IAU 2006/2000A precession-nutation, FK5 star positions with proper motion. Meeus gives abridged series with stated accuracy (sun ≈ 1", moon ≈ 10" in longitude). ERFA (BSD-3) is the clean implementation of the IAU models; SOFA's license forbids a straight port under its names, so ERFA is the reference, not the code.
- The USNO Astronomical Applications API returns GHA, Dec, Hc, Zn, refraction, SD, and PA for every body above the horizon (1800–2050), rise/set/transit and phases, and sidereal times. It is US Government data and the natural oracle. Skyfield (MIT) with DE440s is the second oracle.
- Hipparcos is CC BY-NC; FK5 (VizieR I/149) and the Yale BSC carry no such restriction. The 58 navigational entries are taken from FK5.
- The time domain already has NREL SPA, ΔT (Espenak and Meeus), DUT1 input, leap seconds, and the tzdb.

## Goals / Non-Goals

**Goals:**
- Almanac-grade data (0.1' in GHA and Dec) from open series, verified against USNO.
- Sight reduction, moon events, and survey azimuths that reproduce Bowditch's and USNO's numbers.

**Non-Goals:**
- JPL ephemeris precision. Any almanac table reproduction.

## Decisions

### C1. One astronomy crate, abridged open series, stated accuracy
`gp-astro` implements: VSOP87 series truncated to Meeus's abridged terms for the sun (accuracy ≈ 1") and the four navigational planets (≈ 2" to 10" per Meeus's tables), ELP-2000/82 abridged (≈ 10" longitude, 4" latitude), IAU 2006 precession and 2000A nutation (the full 2000A series is 1,365 terms; the 2000B 77-term series, accurate to 1 mas, is used, named in `meta.model`), aberration, and apparent-to-topocentric reduction. Stars: FK5 J2000 positions and proper motions for the 57 navigational stars and Polaris, precessed and nutated to date. ΔT and DUT1 come from the time domain. Each body's declared accuracy is in the manifest and `meta.accuracy`.
- *Alternative: bundle a JPL ephemeris (DE440s is 17 MB).* Rejected for size and because 0.1' needs no more than the series.
- *Alternative: port ERFA wholesale.* Only the IAU model routines are needed; they are re-derived from the IERS Conventions tables and checked against ERFA's outputs, which the BSD license permits.

### C2. Almanac tool: compute, do not tabulate
`navigation.celestial.almanac` returns, for one instant (UT1 or UTC with ΔT and DUT1 as the sun tool takes them), GHA and Dec for the sun, moon (with HP and SD), Venus, Mars, Jupiter, Saturn, GHA Aries, and SHA and Dec for the 57 stars and Polaris, plus the equation of time and the moon's phase. One instant per call; the batch runner covers a day. The page says plainly that the Nautical Almanac is the governing publication and that these values are computed.

### C3. Sight reduction by the intercept method
Inputs: body, UT date and time, assumed position (or DR, with the assumed-position rule applied and shown), observed altitude Ho (or sextant altitude hs with the correction tool chained). Output: GHA, Dec, LHA, Hc, Z, Zn (with the LHA/hemisphere rule stated), intercept a in nm with Toward/Away, and the line of position for the map. Formulas: Bowditch Appendix B (Hc = asin(sin L sin d + cos L cos d cos LHA); Z by the atan form, quadrant resolved). The Deneb sight of Bowditch §1906 (a = 11.7 nm Toward, Zn 063.3°) is the primary vector; the sun and moon sights of §1907–1908 and the 2002 edition's Spica, Kochab, and sun examples are further vectors.

### C4. Altitude corrections as named steps
`altitude-correction` takes hs, index correction, height of eye, body and limb, temperature and pressure, and (for the moon) HP; it returns ha and Ho with each step shown: dip 1.76'√h_m (0.97'√h_ft), refraction by Bennett 1982 with the temperature-pressure factor, semidiameter (sun from the computed distance; moon from HP), parallax in altitude (moon PA = HP cos ha; sun 0.15'; planets per their HP), and the low-altitude note below 10°. The Nautical Almanac's own correction tables are not reproduced; agreement with them is a test, not a feature.

### C5. Moon events by USNO's definitions
`time.moon.rise-set` finds rise, set, and transit for a local date and zone by iterating the apparent altitude of the moon's center against −(34' + SD − HP) (USNO: upper limb, 34' refraction), with explicit states when there is no event that day (`POLAR_MOON` as an info code, mirroring the sun tool's states) and the day-after-day drift explained. `time.moon.phase` returns the illuminated fraction (Meeus ch. 48), phase angle, age, and the next new, first-quarter, full, and last-quarter instants (Meeus ch. 49, ≈ 2 min). Oracle: USNO `rstt/oneday` and `moon/phases`, with Skyfield second.

### C6. Sidereal time and calendar tools
`time.scale.sidereal` returns GMST, GAST, LMST, LAST, ERA, and the equation of the equinoxes by IAU 2006/2000A (Capitaine, Wallace & Chapront 2005; IERS Conventions 2010 ch. 5), with Meeus's IAU 1982 value shown beside and the ≈ 0.1" difference stated. `time.scale.unix-time` converts Unix seconds ↔ UTC by POSIX.1-2024 §4.19's expression (86,400 s per day, leap seconds not counted, stated), accepting seconds, milliseconds, and microseconds with the magnitude detected and labeled `INPUT_NORMALIZED`. `time.scale.iso-week` converts a date ↔ ISO week date (ISO 8601-1:2019: weeks start Monday; week 1 holds January 4; 53-week years when the year starts on a Thursday or a leap year starts on a Wednesday).

### C7. Survey azimuths by the hour-angle method
`survey.astro.solar-azimuth` and `polaris-azimuth` take the observation (UT1 instant from a time source with its offset, the observer's geodetic position, the horizontal angle from the reference mark to the body, and, for the sun, which limb was observed) and return the body's azimuth Z = atan2(−sin t, cos φ tan δ − sin φ cos t) (from north, t = LHA), the semidiameter correction Δ = SD / cos h for a limb observation, the azimuth of the line, and a note that the result is astronomic, differing from geodetic azimuth by the Laplace correction (deflection of the vertical), which the reader must supply or ignore knowingly (warning `ASTRONOMIC_AZIMUTH`, accuracy). Multiple pointings are averaged with their spread reported. Sources: Bowditch chapter 15 for the formulas and the Polaris example; Ghilani & Wolf's astronomic-observation appendix for the surveying conventions (cited, not reproduced); the NGS astronomic-azimuth course outline.

### C8. Verification
- Almanac: USNO celnav API outputs for 200 instants between 1990 and 2050 and 20 positions: GHA and Dec within 0.1' for sun, planets, Aries, and stars; moon within 0.2' (the ELP abridgment), HP within 0.1'; Skyfield DE440s as the second oracle to the same tolerance. Note (2026-10-20): JPL Horizons (DE441) stands in for Skyfield as the second oracle, since it needs no ephemeris file in the repository. USNO's Venus departs from Horizons by up to 0.4' within weeks of inferior conjunction while the VSOP87 places match Horizons within 1.6"; the USNO check holds Venus to 0.5' and Horizons holds every body to 2". USNO's Moon GHA likewise drifts from Horizons by about 0.005' a year (+0.02' in 1995, −0.18' in 2041, checked at those instants), likely its predicted ΔT, so the 0.2' Moon tolerance against USNO cannot hold to 2050 for any series that matches JPL; the ELP 2000-82B Moon matches Horizons within 0.7", and the USNO check holds it to 0.25'. Stars (2026-10-21): Hipparcos (van Leeuwen 2007) in place of FK5, since FK5 Part I has no Sabik and its Rigil Kentaurus is 0.07' from USNO's; reduced with parallax, light deflection, and aberration, they match ERFA within 0.015" and USNO within 0.008'.
- Sight reduction: Bowditch §1906–1908 to the printed 0.1' and 0.1°; the 2002 examples; property: Hc at the body's geographic position is 90° and the intercept of a true Ho is 0.
- Corrections: Bennett vs the Nautical Almanac's altitude-correction table entries read manually for 20 altitudes (test data, not shipped), within 0.1'.
- Moon: USNO rise/set/transit for 500 place-date pairs within 1 minute (USNO rounds to the minute), phases within 2 minutes, illumination within 0.5%.
- Sidereal: ERFA `gmst06`, `gst06a`, `era00` to 1e-9 rad; USNO `siderealtime` to 0.001 s.
- Unix and ISO week: POSIX expression round trips over 1970–2100; the well-known ISO week anchors (2026-01-01 is week 2026-W01-4; 2027-01-01 is 2026-W53-5).
- Survey azimuths: the Bowditch chapter 15 Polaris example (359.2° tabulated for 2016-02-23 04:21:15 UT at 29°31.0'N 074°30.0'W) and USNO celnav Zn for the sun's center, with the limb correction checked by its formula.

## Tool inventory

| Group | Operations | Endpoints | Reference | Independent vector |
|---|---|---|---|---|
| `navigation.celestial` | almanac, sight-reduction, altitude-correction, polaris-latitude, sun-azimuth-check | 5 | Bowditch ch. 15, 19, App. B; Meeus; IERS 2010; Bennett 1982 | Bowditch §1906–1908; USNO API |
| `time.moon` | position, rise-set, phase | 3 | Meeus ch. 47–49, 15; USNO definitions | USNO rstt and phases API |
| `time.scale` | sidereal, unix-time, iso-week | 3 | Capitaine 2005; IERS 2010; POSIX.1-2024; ISO 8601-1 | ERFA; USNO; POSIX round trips |
| `survey.astro` | solar-azimuth, polaris-azimuth | 2 | Bowditch ch. 15; Ghilani & Wolf appendix; NGS outline | Bowditch §1504; USNO Zn |
| **Total** | **13** | **13** | | |

The navigation rollup gains 5, time gains 6, survey gains 2.

## Risks / Trade-offs

- **[Series size pushes a module over budget]** → `gp-astro` is measured first; the planet series are the largest and can be trimmed to the 0.1' need with the accuracy declared.
- **[Readers expect the almanac's exact digits]** → Tolerances are stated on the page; the Nautical Almanac is named as governing; the oracle agreement is published on `/verification`.
- **[ΔT drift after the table's last entry]** → The existing ΔT table and `UT1_APPROXIMATED` warning carry over; dates beyond the table extrapolate with the warning.

## Migration Plan

Not applicable. New tools start experimental.

## Open Questions

- Whether the full IAU 2000A nutation (1,365 terms) is needed for the stated tolerance or 2000B suffices; measured against ERFA in task 2 and recorded in the derivation note.
