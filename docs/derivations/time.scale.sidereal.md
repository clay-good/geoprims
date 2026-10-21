<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Sidereal time (`time.scale.sidereal`)

## Method

Sidereal time follows the stars rather than the Sun. The IERS Conventions (2010) define it from the Earth rotation angle (ERA), a linear function of UT1. Greenwich mean sidereal time (GMST) adds a polynomial in TT that carries the precession of the equinox (IAU 2006). Greenwich apparent sidereal time (GAST) adds the equation of the equinoxes: the nutation in longitude Δψ times the cosine of the mean obliquity, plus small complementary terms. The tool evaluates Δψ from the IERS Table 5.3a series (IAU 2000A_R06, 1,320 periodic terms and 38 terms in t) and the complementary terms from Table 5.2e, with the fundamental arguments of equations 5.43 and 5.44. The tables are generated into the code from the IERS files by `tools/codegen/iers_tables.py`. UT1 is UTC plus the DUT1 the reader gives (UT1_APPROXIMATED without it). TT is UTC plus TAI − UTC plus 32.184 s from 1972, and UT1 plus a dated ΔT estimate before that. Local sidereal time adds the east longitude. Meeus's IAU 1982 GMST is shown beside for comparison.

## Equations

- ERA = 2π (UT1 Julian-day fraction + 0.7790572732640 + 0.00273781191135448 Tu), Tu = JD(UT1) − 2451545.0 (eq. 5.15).
- GMST = ERA + (0.014506 + 4612.156534 t + 1.3915817 t² − 0.00000044 t³ − 0.000029956 t⁴ − 0.0000000368 t⁵)″, t in TT Julian centuries (eq. 5.32).
- εA = 84381.406″ − 46.836769″ t − 0.0001831″ t² + 0.0020034″ t³ − 0.000000576″ t⁴ − 0.0000000434″ t⁵ (eq. 5.40).
- Δψ = Σ (A sin φ + A″ cos φ) + t Σ (A′ sin φ + A‴ cos φ) over Table 5.3a, φ an integer combination of the 14 fundamental arguments.
- GAST = GMST + Δψ cos εA + Σ (C_s sin φ + C_c cos φ) + t Σ (…) over Table 5.2e.
- LMST = GMST + λ, LAST = GAST + λ, λ east longitude.
- Meeus 12.4 (IAU 1982): θ₀ = 280.46061837° + 360.98564736629° (JD − 2451545) + 0.000387933° T² − T³/38710000.

## Symbols and units

Angles in radians internally; sidereal times shown as hours, minutes, and seconds to the millisecond and as decimal hours; ERA in degrees; the equation of the equinoxes and the IAU 1982 difference in seconds of time.

## Domain

A UTC time from about 3500 BC to AD 7400; DUT1 within ±0.9 s (INVALID_INPUT otherwise); longitude within ±360°.

## Approximations

The IERS tables are used whole, so the model error is that of IAU 2006/2000A itself. ERFA uses MHB2000's slightly rounded constants for two Delaunay arguments and the longitude of Neptune; the Conventions put the difference at about 2 µas, and the measured GAST difference is 1e-11 rad. Without DUT1, UT1 is taken as UTC, which can move every result by up to 0.9 s of UT1 (13.5″). Before 1972 TT comes from a ΔT estimate, which is uncertain by seconds in early centuries.

## Worked example

- sourcePublisher: NumFOCUS (ERFA, derived from the IAU SOFA library)
- sourceTitle: ERFA era00, gmst06, and gst06a, through pyerfa
- sourceEdition: ERFA 2.0.1 (pyerfa 2.0.1.5)
- sourceLocator: 240 instants from 1972 to 2050 with random DUT1, UTC to TT by utctai and taitt (tests/data/erfa_sidereal.json)
- independent: yes
- inputs: each instant's UTC time and DUT1
- outputs: ERA and GMST within 6e-14 rad and GAST within 1e-11 rad of ERFA; at J2000.0 (2000-01-01T12:00 UT1) GMST is 18h 41m 50.549s (IAU 2006; 50.548 s by IAU 1982) and ERA 280.4606°
- tolerance: 1e-9 rad (the spec's)
- verifiedBy: `core/crates/gp-time/tests/sidereal.rs` `matches_erfa_within_1e9_rad` and golden vectors v002 to v022
- verifiedOn: 2026-10-20

## Differential tests

- `core/crates/gp-time/tests/sidereal.rs` `matches_erfa_within_1e9_rad`: ERA, GMST, and GAST against ERFA at 240 instants, worst 5.7e-14, 5.7e-14, and 9.7e-12 rad.
- `core/crates/gp-time/tests/sidereal.rs` `matches_usno_within_a_millisecond`: GMST, GAST, LMST, and LAST against USNO's sidereal-time API (fetched 2026-10-20, `tests/data/usno_sidereal.json`) at 40 random UT1 instants and places from 1990 to 2049, worst 0.0005 s, the two outputs' rounding; the equation of the equinoxes within 0.0002 s.
- `tools/vectors/gen_sidereal.py`: the J2000.0 scenario and 21 of the ERFA instants within 1e-9 rad; 22 vectors in `core/vectors/time.scale.sidereal.jsonl`.

## Invariants

- `core/crates/gp-time/tests/sidereal.rs` `j2000_and_local_sidereal_time`: the J2000.0 scenario; local mean sidereal time at 74°30′W is GMST less 4h 58m 00s; UT1_APPROXIMATED appears without DUT1 and not with it; the IAU 1982 value within 10 ms of IAU 2006 near J2000; DUT1 beyond ±0.9 s refused.
