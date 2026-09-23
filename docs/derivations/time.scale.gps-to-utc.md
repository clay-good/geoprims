<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# GPS week and second to UTC (`time.scale.gps-to-utc`)

## Method

This is the inverse of `time.scale.gps-week`, and it inherits the same leap-second table and the same three constants: the GPS epoch of 1980-01-06T00:00:00 UTC, the 604,800-second week, and GPS = TAI − 19 s.

Going this way has one extra problem, and it is the reason the tool exists in its own right. A week number read out of a legacy navigation message is ten bits, so it could be any of three weeks 19.6 years apart. A bare week below 1,024 is therefore **ambiguous**, and the tool refuses it rather than picking an era: the error names the three eras and what years they cover. Give an era and the full week is era × 1,024 + the 10-bit week. Give a week of 1,024 or more and it names itself, and an era alongside it is refused rather than added — adding it would move the answer 19.6 years without saying so. Week 0 is the GPS epoch and is also a 10-bit week, so like any week below 1,024 it has to be given with an era.

Subtracting the offset is the rest. GPS time is converted to TAI, TAI to UTC through the table, and the result is an RFC 3339 instant. An instant that falls inside an inserted leap second prints as `23:59:60`, which is a real UTC time and not an overflow.

## Equations

- full week = 10-bit week + 1,024 × era, when an era is given.
- s = week × 604,800 + seconds of week, seconds of GPS time since the epoch.
- UTC = GPS epoch + s − (GPS − UTC at that instant), with GPS − UTC = (TAI − UTC) − 19 s.

## Symbols and units

`week` is a full or 10-bit week number, `seconds_of_week` is 0–604,799, and `era` is 0, 1 or 2. Out come `utc`, the resolved `gps_week`, and `gps_minus_utc` in seconds.

## Domain

Weeks from 0 to the end of the embedded leap-second table's coverage; seconds of week in [0, 604,800). A week below 1,024 without an era is refused as ambiguous, which is an error rather than a warning because there is no defensible default. A week of 1,024 or more given *with* an era is refused too, for the mirror-image reason: it is already a full week, and an era would double-count.

## Approximations

None in the arithmetic. As with the forward direction, an instant past the embedded table's expiry could move by a second if a leap second is announced, and the result carries `LEAP_SECOND_TABLE_EXPIRED` when that applies.

## Worked example

- sourcePublisher: Internet Assigned Numbers Authority (tz database); U.S. Space Force
- sourceTitle: IANA tzdb `leapseconds` file; IS-GPS-200, Navstar GPS Space Segment/Navigation User Interfaces
- sourceEdition: IANA tzdb 2026c; IS-GPS-200 as cited in the tool's references
- sourceLocator: the tzdb leap-second list; GPS time = TAI − 19 s; the 10-bit week rolls over every 1,024 weeks
- independent: yes
- inputs: 22 week-and-second pairs, given as full weeks where unambiguous and as 10-bit weeks with an era otherwise, spanning the GPS epoch, both rollovers and three leap-second boundaries
- outputs: the UTC instant and the resolved full week
- tolerance: exact
- verifiedBy: golden vectors v001 to v022, run by the core on every build
- verifiedOn: 2026-09-23

Every vector here is the round trip of a `time.scale.gps-week` vector: the UTC time that tool was given is the UTC time this one must return. The expected week and second come from the IANA tzdb leap-second table read by `tools/vectors/gen_time_gps_tz.py`, not from the core, so the pair being each other's inverse is checked against an outside table rather than against themselves.

Writing those vectors found the ambiguity rule working exactly as it should: a generated case with a full week of 0 and no era was refused, because week 0 could be 1980, 1999 or 2019. The generator now supplies an era whenever the week is below 1,024.

## Differential tests

- `tools/vectors/gen_time_gps_tz.py`: 16 of the 22 vectors, from the IANA tzdb leap-second file
- `core/crates/gp-time/tests/time.rs`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/time.scale.gps-to-utc.jsonl`: 22 vectors, both ways of naming a week

## Invariants

- `core/crates/gp-time/tests/time.rs` `gps_to_utc_invariants`: the two tools are exact inverses — a UTC time through `gps-week` and back through this one returns the same instant, at the epoch, at both rollovers and either side of the 2017 leap second; week 0 second 0 with era 0 is the GPS epoch itself; a 10-bit week with its era gives the same instant as the full week does, for all three eras; a bare week below 1,024 is refused with an error naming the eras, and the same week with an era is accepted, so the rule is a refusal and not a silent default; and a week of 1,024 or more is accepted alone and refused with an era, which is the same rule read from the other end
