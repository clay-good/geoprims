<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# The four aviation nights (`time.sun.aviation-nights`)

## Method

For the evening of the local date and the next morning, find sunset and sunrise (−0.833°) and the end and beginning of civil twilight (−6°) with the same SPA-based crossing solver as the sunrise tool, one side each. Then apply each regulation's definition. A landing time, if given, is checked against logging night and passenger currency. A time before local noon is read as the next morning.

## Equations

- Position lights (14 CFR 91.209): sunset to sunrise.
- Logging night (14 CFR 1.1): end of evening civil twilight to beginning of morning civil twilight.
- Passenger currency (14 CFR 61.57(b)): sunset + 1 h to sunrise − 1 h, or none when the night is shorter than 2 hours.
- Part 107 civil twilight (14 CFR 107.29(c)): 30 minutes after sunset and 30 minutes before sunrise. With `alaska: yes`, the civil twilight period instead.

## Symbols and units

Latitude and longitude in degrees, the local date, and a UTC offset or IANA zone. Windows are shown in local time and Zulu, each dated, to the minute.

## Domain

Any place and date. When the sun does not set, does not rise, or civil twilight lasts all night, each affected window says "none" and why.

## Approximations

Civil twilight is the sun's center 6° below the horizon (the CIVIL_TWILIGHT_APPROXIMATED warning). The Air Almanac is the legal source for 14 CFR 1.1 and can differ by a minute. Sunrise and sunset use standard refraction.

## Worked example

- sourcePublisher: United States Naval Observatory, Astronomical Applications Department, with 14 CFR 1.1, 61.57(b), 91.209, and 107.29
- sourceTitle: Sun and Moon Data for One Day (API rstt/oneday), with the regulations' definitions
- sourceEdition: API 4.0.1, retrieved 2026-09-18
- sourceLocator: Denver (39.7392, −104.9903), 2026-06-21, UTC−6: sunset 20:31, end of civil twilight 21:04, next sunrise 05:32, next beginning of civil twilight 05:00
- independent: yes
- inputs: 39.7392, −104.9903, date 2026-06-21, offset −06:00
- outputs: passenger currency from 21:31, logging night 21:04 to 05:00, position lights 20:31 to 05:32, Part 107 evening 20:31 to 21:01
- tolerance: 1 minute
- verifiedBy: golden vectors v001 to v005, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-time/tests/usno_nights.rs`: 100 places (60 in the conterminous US, 20 in Alaska, 20 elsewhere) on seeded dates. Every window is built from USNO's times by its regulation, and all 491 are within a minute.
- `tools/vectors/gen_usno_sun.py`: regenerates that fixture with `--nights`
- `core/vectors/time.sun.aviation-nights.jsonl`: 22 vectors, including 16 landings 15 minutes either side of the currency start at USNO-timed places

## Invariants

- `core/crates/gp-time/tests/time.rs` `aviation_nights_invariants`: logging night lies inside sunset-to-sunrise, passenger currency starts and ends 1 hour in, and the Part 107 windows are the 30 minutes after sunset and before sunrise
