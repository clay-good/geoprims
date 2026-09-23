<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Julian date, MJD, and day of year (`time.scale.julian-date`)

## Method

The Julian date is a continuous count of days from a fixed instant, and everything here follows from where that instant is: JD 0.0 is noon UT on 1 January 4713 BC in the proleptic Julian calendar. The consequence people trip over is the **half day**. A Julian date rolls over at noon, not at midnight, so midnight UTC is always a `.5`. The tool works from the Unix epoch, where JD 2440587.5 is 1970-01-01T00:00:00Z, and adds the elapsed days.

The modified Julian date moves the origin to 1858-11-17T00:00:00Z and drops the half day: MJD = JD − 2400000.5, so an MJD *does* roll over at midnight. The two conventions differ in both origin and rollover, which is why they are reported together rather than left to the reader.

Day of year is the ordinal day, 1 on 1 January, so 366 on 31 December of a leap year. The Gregorian leap rule is the whole rule: divisible by four, except centuries, except centuries divisible by 400. 1900 was not a leap year and 2000 was.

The RINEX daily file name follows the observation-file convention: four station characters, the three-digit day of year, a session character, and a two-digit year with the `o` type.

## Equations

- JD = 2440587.5 + (seconds since 1970-01-01T00:00:00Z) / 86,400.
- MJD = JD − 2400000.5.
- day of year = the ordinal day of the UTC date, 1 for 1 January.
- Leap year: (y mod 4 = 0 and y mod 100 ≠ 0) or y mod 400 = 0.

## Symbols and units

`utc` is an RFC 3339 time; `jd` and `mjd` are day counts, `day_of_year` an integer 1–366, `year` the UTC year, and `rinex_name` a file name. Give a JD or an MJD instead of a UTC time and the UTC time comes back.

## Domain

Any date the proleptic Gregorian calendar covers, before the Unix epoch as well as after — 1969 gives a JD below 2440587.5, which is correct and not an error.

## Approximations

UTC is not a uniform time scale: a leap second makes a UTC day 86,401 seconds long, and the JD returned here is a count of 86,400-second days from the UTC epoch, the convention every JD-from-UTC converter uses. So within a day containing a leap second the JD is off by up to one second from a strictly TAI-derived count. For the purposes a JD is used for — a day number, a RINEX file name, an epoch label — that does not matter, and it is stated here rather than implied.

## Worked example

- sourcePublisher: U.S. Naval Observatory, Astronomical Applications Department; Python Software Foundation
- sourceTitle: Julian Date Converter; Python `datetime` (proleptic Gregorian calendar)
- sourceEdition: USNO online data service (2024); CPython 3.13
- sourceLocator: JD 2451545.0 is 2000 January 1 at 12:00 UT; MJD = JD − 2400000.5
- independent: yes
- inputs: 24 times, including the Unix epoch, the GPS epoch, J2000 noon, 29 February 2000 and 1 March 1900, 28 February 2100, the last day of a leap year, and a date before the Unix epoch
- outputs: JD, MJD, day of year and year in each case
- tolerance: 1e-9 days on JD and MJD, exact on day of year and year
- verifiedBy: golden vectors v001 to v024, run by the core on every build
- verifiedOn: 2026-09-23

The half-day convention is checked at the point where it can only be right or obviously wrong: J2000, where the USNO's own published value is JD 2451545.0 at **noon** on 1 January 2000, and the MJD for the same instant is 51544.5. A converter that put the rollover at midnight would return 2451545.0 twelve hours out, and the two vectors either side of it would disagree by half a day.

The calendar comes from Python's `datetime`, a separate implementation of the proleptic Gregorian rules. The century cases are the ones worth having: 1900-03-01 and 2100-02-28 both sit immediately after a February that a naive divisible-by-four rule would have given 29 days.

## Differential tests

- `tools/vectors/gen_time_scale.py`: 17 of the 24 vectors, from Python's `datetime` and the JD definition
- `core/crates/gp-time/tests/time.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/time.scale.julian-date.jsonl`: 24 vectors, both century rules, a leap day, a pre-epoch date, and times through the day

## Invariants

- `core/crates/gp-time/tests/time.rs` `julian_date_invariants`: the USNO's published J2000 value holds exactly — JD 2451545.0 at noon on 2000-01-01, and MJD 51544.5 at the same instant; midnight UTC always lands on a JD ending in .5 and an MJD ending in .0, which is the half-day convention stated as a property rather than a single case; MJD is JD − 2400000.5 for every vector; a day later is exactly one more JD; day of year is 1 on 1 January and 365 or 366 on 31 December according to the Gregorian leap rule, with 1900 not a leap year and 2000 one; and giving back a JD or MJD returns the UTC time it came from
