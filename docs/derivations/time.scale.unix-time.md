<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Unix time converter (`time.scale.unix-time`)

## Method

POSIX.1-2024 §4.19 defines Unix time from a UTC calendar time by a C expression that counts every day as exactly 86,400 seconds. The tool uses the same count as whole days since 1970-01-01 times 86,400 plus the seconds of the UTC day, on the proleptic Gregorian calendar, which gives the same value as the POSIX expression for every date from 1970. Converting back splits the count into days and seconds of the day. A pasted count is read as seconds, milliseconds, microseconds, or nanoseconds from its number of digits (up to 11, 12 to 14, 15 to 17, more), and INPUT_NORMALIZED names the unit read unless the reader chose one. The result also gives the Julian date and, from 1980-01-06, the GPS week and seconds of the week by the GPS week tool's rules, which do count leap seconds.

## Equations

- Unix seconds = days since 1970-01-01 × 86,400 + seconds of the UTC day.
- JD = 2440587.5 + Unix seconds / 86,400.
- GPS seconds = (days since 1980-01-06) × 86,400 + seconds of the day + (TAI − UTC) − 19 s; week = floor(GPS seconds / 604,800).

## Symbols and units

Unix time in seconds (or the unit read or chosen); UTC as ISO 8601 with Z; JD in days.

## Domain

Years 0001 to 9999; a UTC time inside a leap second (23:59:60) is INVALID_INPUT, because Unix time has no value for it, with the hint to use TAI or GPS time. Before 1970 POSIX leaves the value undefined; the tool extends the day count backward and says so.

## Approximations

None: integer day arithmetic, exact to the millisecond. Unix time is not a uniform time scale; across a leap second it repeats or skips nothing because the leap second simply has no count.

## Worked example

- sourcePublisher: IEEE and The Open Group
- sourceTitle: POSIX.1-2024, Base Definitions §4.19, Seconds Since the Epoch
- sourceEdition: IEEE Std 1003.1-2024
- sourceLocator: §4.19; the add-celestial-and-moon scenario 1760832000000
- independent: no
- inputs: 1760832000000
- outputs: read as milliseconds, 2025-10-19T00:00:00Z, with INPUT_NORMALIZED (the spec printed 2026; corrected in the spec)
- tolerance: exact
- verifiedBy: golden vector v001 and `core/crates/gp-time/tests/calendar.rs` `unix_time_scenarios`
- verifiedOn: 2026-10-20

## Differential tests

- `tools/vectors/gen_calendar.py unix`: 20 vectors from Python's datetime (an independent POSIX time implementation), from the epoch and negative times to 2100, the 2038 rollover, a UTC offset, milliseconds and nanoseconds, and the leap-second refusal; `core/vectors/time.scale.unix-time.jsonl`.

## Invariants

- `core/crates/gp-time/tests/calendar.rs` `unix_time_scenarios`: the epoch; milliseconds read with INPUT_NORMALIZED; an explicit unit; the leap-second refusal with its hint; the seconds either side of the 2016 leap second one Unix second apart; the GPS epoch as week 0, second 0; a UTC offset applied.
- `core/crates/gp-time/tests/calendar.rs` `unix_time_round_trips_1970_to_2100`: Unix time to UTC and back for instants across 1970 to 2100.
