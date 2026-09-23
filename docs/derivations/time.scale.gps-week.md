<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# GPS week and seconds of week (`time.scale.gps-week`)

## Method

GPS time is a uniform count of seconds that has never stopped for a leap second. UTC has stopped 27 times. So converting between them is not arithmetic on a formula — it needs the published leap-second table, and that is the whole difficulty.

Three constants fix the scale. The GPS epoch is 1980-01-06T00:00:00 UTC. GPS time runs at the rate of TAI, offset so that GPS = TAI − 19 s, which is what made GPS time equal UTC at the epoch: TAI − UTC was exactly 19 s that day. Since then every leap second has widened the gap, so GPS − UTC = (TAI − UTC) − 19, which is 18 s from 2017-01-01 and will change again when the IERS says so.

Weeks are counted from the epoch in 604,800-second blocks. The legacy navigation message carries only ten bits of week number, so it wraps every 1,024 weeks — 19.6 years. Two rollovers have happened, in August 1999 and April 2019. The tool reports the full week, the 10-bit week and the era, so a receiver's raw week number can be placed without guessing which era it came from.

## Equations

- GPS − UTC = (TAI − UTC) − 19 s, from the published leap-second table at that instant.
- s = (UTC − 1980-01-06T00:00:00Z in seconds) + (GPS − UTC).
- week = ⌊s / 604,800⌋; seconds of week = s mod 604,800.
- 10-bit week = week mod 1,024; era = ⌊week / 1,024⌋.

## Symbols and units

`utc` is an RFC 3339 time. Out come `gps_week`, `seconds_of_week`, `week_10bit`, `rollover_era`, `gps_minus_utc` and `tai_minus_utc` in seconds, and `gps_time`.

## Domain

Any UTC time from the GPS epoch onwards that the embedded leap-second table covers. Past the table's expiry the answer is still given but carries a `LEAP_SECOND_TABLE_EXPIRED` warning, because a leap second announced after the table was built would change it by a second.

## Approximations

None in the arithmetic. The one uncertainty is the future: leap seconds are announced roughly six months ahead by IERS Bulletin C, so any instant past the embedded table's expiry could be a second out. That is a fact about the world and the result says so rather than hiding it.

## Worked example

- sourcePublisher: Internet Assigned Numbers Authority (tz database); U.S. Space Force
- sourceTitle: IANA tzdb `leapseconds` file; IS-GPS-200, Navstar GPS Space Segment/Navigation User Interfaces
- sourceEdition: IANA tzdb 2026c; IS-GPS-200 as cited in the tool's references
- sourceLocator: the tzdb leap-second list, accumulated from the 10 s that TAI − UTC stood at on 1972-01-01; GPS time = TAI − 19 s
- independent: yes
- inputs: 24 UTC times, including the GPS epoch, the instants either side of both 1,024-week rollovers, either side of the 2012, 2015 and 2017 leap seconds, and ordinary times in all three eras
- outputs: the full week, seconds of week, 10-bit week, era, and both offsets
- tolerance: exact
- verifiedBy: golden vectors v001 to v024, run by the core on every build
- verifiedOn: 2026-09-23

The reference is not a formula restated but a different table read a different way. `tools/vectors/gen_time_gps_tz.py` parses the IANA tzdb's own `leapseconds` file, accumulates TAI − UTC from the 10 s it stood at when the leap-second system began, and subtracts 19. The core carries its own embedded table. The two agree at every instant tried, including the second either side of three leap seconds, where a table off by one entry would show immediately.

## Differential tests

- `tools/vectors/gen_time_gps_tz.py`: 16 of the 24 vectors, from the IANA tzdb leap-second file
- `core/crates/gp-time/tests/time.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/time.scale.gps-week.jsonl`: 24 vectors, both rollovers and three leap-second boundaries

## Invariants

- `core/crates/gp-time/tests/time.rs` `gps_week_invariants`: at the GPS epoch the week, seconds of week and GPS − UTC are all zero, which is the definition and the one instant where all three coincide; GPS − UTC is exactly TAI − UTC − 19 everywhere; the offset is 18 s from 2017-01-01 and 17 s the second before, so the table is read at the right boundary and not a day either side; both 1,024-week rollovers land where they should, week 1024 becoming 10-bit week 0 in era 1 and week 2048 doing the same in era 2; the full week is always era × 1,024 + the 10-bit week; seconds of week stay within [0, 604,800); and a week later is exactly one more week at the same second
